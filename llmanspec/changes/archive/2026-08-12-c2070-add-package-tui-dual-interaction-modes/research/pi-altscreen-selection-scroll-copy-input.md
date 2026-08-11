# Research: Pi `TuiAltScreen` 选区 / 越界滚动 / 松手复制 / 输入区特例

> 日期：2026-08-11
> Change：`c2070-add-package-tui-dual-interaction-modes`（Mode B 实现前）
> 一手：`/home/l8ng/Projects/__straydragon__/pi`（下文路径相对该仓库，除非另注）
> 范围：只读；不改 proposal/specs/应用代码

## 0. 结论摘要

| 主题 | Pi 事实 |
|---|---|
| Mode A / B 映射 | A ≈ `TuiMainScreen`/`regular`；B ≈ `TuiAltScreen`/`fullscreen`（应用视口+应用选区） |
| 启动 | `?1049h` alt-buffer；mouse 默认开；多路复用器用 **1002**，直连用 **1002+1003** |
| 选区 | anchor/focus + character/word/line；拖选 / 双击词 / 三击行；松手 **OSC52** + flash「Copied!」 |
| 越界续选 | 指针贴 ScrollView 可见顶/底 → `setInterval(50ms)` 调 `ScrollView.scrollBy` |
| 输入框 | **无**显式「排除 Input 行」API；靠 **dock 在 ScrollView 外** 做高亮裁剪；鼠标 **consume**，**不** click-to-focus Editor |
| 折叠 hit | **Pi 未实现**（AltScreen 无 fold 点击） |
| 优先级 | 右键粘贴(Win) → scrollbar 拖 → 选区/OSC8；overlay 时禁 scrollbar 与 ScrollView 锚定选 |

---

## 1. Mode A vs Mode B：类型与入口

| xylitol 称呼 | Pi 类型 | `mode` | 入口 |
|---|---|---|---|
| Mode A（inline / emulator-owned） | `TuiMainScreen` | `"regular"` | `createInteractiveTui` 非 fullscreen 分支 |
| Mode B（alt-screen / application-owned） | `TuiAltScreen` | `"fullscreen"` + `ViewportTUI` | `tuiMode === "fullscreen"` |

- 联合类型：`TuiMode = "regular" | "fullscreen"`（`packages/tui/src/tui.ts` → `TuiMode`）。
- 工厂：`createInteractiveTui`（`packages/coding-agent/src/modes/interactive/interactive-mode.ts`）— fullscreen 时 `new TuiAltScreen(..., { openUrl, onRightClickPaste, search styles })`，否则 `new TuiMainScreen`。
- 设置：`SettingsManager.getTuiMode` / `setTuiMode`（同文件族 `settings-manager.ts`）；非法值回落 `regular`。
- 运行时换栈：`InteractiveMode.switchTuiMode` — stop+preserveScreen → 重建 renderer → `mountInteractiveTui`；fullscreen 挂 `setLayoutRoot(fullscreenLayoutRoot)`。
- 布局（产品）：`transcriptScrollView`（primary ScrollView）+ 底部 `dock`（pending/status/widgets/**editor**/footer）（`interactive-mode.ts` `init` 附近）。

`TuiMainScreen`：**无** mouse tracking / 应用选区字段（终端 scrollback + 原生选区）。本文件以下仅 AltScreen。

---

## 2. AltScreen 启动 / teardown

**启动** `TuiAltScreen.beforeTerminalStart`（`packages/tui/src/tui-alt-screen.ts`）：

1. 清选区/scrollbar/search 相关状态；`altScreenActive = true`。
2. 写入：`ENTER_ALT_SCREEN` (`\x1b[?1049h`) + `DISABLE_AUTOWRAP` +（若 `mouseEnabled`）mouse 序列 + `\x1b[2J\x1b[H\x1b[?25l`。
3. Mouse 默认：`options.mouse ?? true`（`TuiAltScreenOptions.mouse` / 构造）。
4. 模式选择：
   - 多路复用（`TMUX` / `ZELLIJ` / `STY` 或 `TERM` 以 `tmux`/`screen` 开头）→ `ENABLE_BUTTON_MOTION_MOUSE`：`1000h` + **`1002h`** + `1004h` + `1006h`（**无 1003**）。
   - 否则 → `ENABLE_ALL_MOTION_MOUSE`：同上并加 **`1003h`**（全运动，便于 scrollbar hover 等）。
5. 理由注释：multiplexer 转发每个 motion 会卡；button-motion 仍够点击/滚轮/选区/scrollbar。

**Teardown**：

- `beforeTerminalStop`：关 search；停 auto-scroll / scrollbar；若 mouse 开则 `DISABLE_MOUSE`（`1006l…1000l`）；Kitty 图清理；**尚不** `EXIT_ALT_SCREEN`。
- `afterTerminalStop`：`EXIT_ALT_SCREEN`（`?1049l`）；可选把最后文档打回主屏，或 `preserveScreen` 仅退 buffer。

测证：`packages/tui/test/tui-alt-screen.test.ts`「uses button-motion tracking inside terminal multiplexers」。

---

## 3. 选区状态机

**状态**（`TuiAltScreen` 私有字段）：

| 字段 | 含义 |
|---|---|
| `selectionAnchor` / `selectionFocus` | `SelectionPoint{row,col,scrollView?,boundary?}` |
| `selectionGranularity` | `"character" \| "word" \| "line"` |
| `selectionInitialRange` | 双击/三击初始词/行，用于拖扩展 |
| `selectionPressActive` / `selectionDragged` | 按住中 / 是否拖过 |
| `lastClick` | 多击计数（500ms、`DOUBLE_CLICK_INTERVAL_MS`） |
| `pressedUrl` | 按下时 OSC8 URL（仅 character 单击路径） |

**入口** `handleSelectionMouseEvent`（仅主按钮 `(button&3)===0`）：

| 事件 | 行为 |
|---|---|
| press | 取 `getScrollViewsAt` 顶层 ScrollView（无 overlay）；锚点；`getClickCount` → 2=词、3=行、否则字符；记 `pressedUrl`（词/行选时清空） |
| motion (`button&32`) | 若 press 中：`selectionDragged=true`；`updateSelectionFocus`；`updateSelectionAutoScroll` |
| release | 停 auto-scroll；若未拖且点落原细胞且有 `pressedUrl` → `openUrl` 并清选；否则 **`copySelectionToClipboard`** |

**粒度**：

- 词：`Intl` word segmenter（`getWordSelection`）+ `boundary:true` 终点。
- 行：col 0 → 行宽，`boundary:true`。
- 词/行拖：`updateSelectionFocus` 以初始 range 为枢，扩展到指针处词/行边界（非半词）。
- 空选（anchor==focus）：`getSelectionBounds` → `undefined`（不复制、不高亮）。

测证：同文件 double/triple click、CJK/emoji grapheme、focus 丢失用例。

---

## 4. 越界续选 / auto-scroll

`updateSelectionAutoScroll` / `autoScrollSelection`：

| 项 | 行为 |
|---|---|
| 前提 | 选区已绑 `selectionAnchor.scrollView` 且有 layout box |
| 触发 | 指针 `y <= visibleTop` → 方向 −1；`y >= visibleBottom` → +1；否则停 |
| 定时 | `setInterval(..., 50)`，`unref()`；已有 timer 不重复建 |
| 步进 | `scrollView.scrollBy(direction)`；若无法再滚则停 |
| 续选 | 每次滚动后用固定 `selectionDragPointer` 调 `getScrollSelectionPoint` → `updateSelectionFocus` |
| 关系 | **应用 ScrollView**，非终端 scrollback；指针 y 在 box 外会被夹到可见边（`getScrollSelectionPoint`） |

测证：「auto-scrolls and extends a drag selection held at the viewport edge」。

---

## 5. 松手复制与右键粘贴

**复制** `copySelectionToClipboard`：

1. 非空 bounds；源行 = ScrollView `scrollContentLines` 或 `previousScreen`。
2. 按列切片、`stripTerminalSequences`、`trimEnd`，行间 `\n`。
3. **仅 OSC52**：`terminal.write("\x1b]52;c;" + base64 + "\x07")`。
4. `flash("Copied!")`；**不**清选区（完成后高亮可保留；FOCUS_OUT 仅清「按住中」的选）。

**默认**：mouse 开则整条选区路径默认可用；**无**「关松手复制」开关。
**本地剪贴板 API**：**Pi `TuiAltScreen` 未实现**（无 wl-copy/pbcopy 回退）。产品层另有 `copyToClipboard` / `readClipboardText`（`coding-agent` utils），**不**接在松手选区路径。

**右键粘贴**：`handleRightClickPaste` — 需 `onRightClickPaste` 且 **`process.platform === "win32"`**、button 2、非 release。产品实现读本地剪贴板，向 **当前 focus** 组件注入 bracketed paste。非 Win：**等效未启用**。

---

## 6. 输入框 / Editor 特别处理（产品级）

Pi **没有**名为 input-exclude 的选区 API。行为来自布局 + 事件消费：

| 场景 | 行为 | 源 |
|---|---|---|
| 产品 layout | Editor 在 **ScrollView 外** 的 dock | `interactive-mode.ts` fullscreenLayoutRoot |
| 选区始于 transcript | 高亮 `applySelection` 裁到 ScrollView box（`minRow/maxRow/minColumn/maxColumn`）→ **dock/editor 行不进选区高亮** | `applySelection` |
| 拖入 dock | 指针夹在 ScrollView 可见底 → 续选内容底行，**不**选中 editor 文本 | `getScrollSelectionPoint` |
| 按下落在 dock（无 ScrollView） | 用屏幕坐标选 `previousScreen` → **可能**盖住 editor 行（无专用排除） | `getSelectionPoint` 回退 |
| 鼠标与焦点 | 所有解析到的 mouse **`consume: true`**，**不**调用 `setFocus` → **Pi 未实现 click-to-focus Editor**；键盘焦点保持既有 | `handleViewportInput` / `TuiBase.handleTerminalInput` |
| Editor 收鼠标 | **收不到**（被 viewport listener 吃掉）；caret 不随点 | 同上 |
| 滚轮在 dock | 无 ScrollView 命中 → 剩余 delta 落到 primary transcript | `routeWheel` + 测「keeps an explicit dock fixed」 |
| Scrollbar vs 选区 | `handleScrollbarMouseEvent` **先**于选区；命中 thumb 则清选并拖条；拖条期间 **无 OSC52** | `handleViewportInput` 顺序 + 测 |
| 键盘抢占 | 未修饰 `pageUp/Down` 等 `tui.altScreen.*` 在 listener 消费，**遮蔽** editor 同键（注释 intentional） | `keybindings.ts`；Ctrl 修饰仍可到 focus 组件（测） |

结论：**「排除 input」= 布局把 input 放在 ScrollView 外 + 选区高亮裁剪**；不是 hit-test 白名单。若 Mode B 把 Editor 画进同一 ScrollView，需自研排除，**不能**照抄「Pi 有专门 exclude」。

---

## 7. 与点击 / 折叠 / OSC8 优先级

鼠标流水线（`handleViewportInput`）：

```
FOCUS_IN/OUT → wheel → SGR mouse:
  1) handleRightClickPaste (Win, btn2)
  2) handleScrollbarMouseEvent（成功则仍可 update hover，但不再进选区）
  3) else handleSelectionMouseEvent
```

选区内 OSC8（`pressedUrl` + `getOsc8LinkAtColumn` on `previousScreen`）：

- **单击无拖** + release 同点 → `openUrl`，清选，**不**复制。
- **拖过** → 不当作链接，走复制。
- 双击/三击建词行选时不记 URL。

**折叠 / activity fold / starline 三角点击**：**Pi `TuiAltScreen` 未实现**（无 fold hit-test；折叠点击属 xylitol cascade，需在 Mode B 自定优先级，常见 SHOULD：折叠标记消费 click，其余选区）。

Overlay：`hasOverlay()` 时 scrollbar 目标空、press 不绑 ScrollView（屏幕坐标选）。

---

## 8. 对 xylitol-tui Mode B 的可移植切分建议

产品级子系统边界（勿钉死长文件清单）：

| 子系统 | 应对齐的 Pi 行为 | 注意 |
|---|---|---|
| **viewport** | alt-buffer 生命周期；ScrollView 式 app scroll；mouse 1002/1003 策略；teardown Disable | 是否强制 `?1049h` 见 proposal Open Q；建议与 Pi 同族 |
| **selection** | anchor/focus、粒度、拖选、多击、grapheme；高亮 inverse；与 ScrollView 坐标系 | 空选不复制；FOCUS_OUT 取消按住中选 |
| **auto-scroll** | 边沿触发 + 短周期 timer + scrollBy 续 focus | 与 viewport 强耦合，可同模块但接口分离 |
| **clipboard** | 默认松手 OSC52；flash 反馈 | 本地工具为可选增强（Pi AltScreen 无）；可配置但默认开 |
| **input-exclude** | **结构**：transcript ScrollView vs dock(input)；选区高亮裁到 transcript | 另补：是否允许 click-to-focus（Pi 无）；fold hit 优先级（Pi 无，xylitol 要定） |
| **hit-priority** | scrollbar > selection；click-link vs drag-select | 折叠标记插入此链，勿与选区抢同一 press |

建议实现顺序：viewport+mouse → selection+highlight → auto-scroll → OSC52 → dock 裁剪契约 →（cascade）fold/OSC8。

---

## 9. 「Pi 未实现」清单（本调研范围内）

- Mode A 应用内选区 / AltScreen 级 mouse。
- 松手复制的本地剪贴板回退（仅 OSC52）。
- 非 Windows 右键粘贴。
- click 落在 Editor 抢键盘焦点 / 鼠标改 caret。
- 显式「选区排除 input 行」API（靠布局）。
- AltScreen 折叠/三角 hit-test。
- 可关「松手自动复制」的库选项。

---

## 主要源符号索引

| 符号 | 路径 |
|---|---|
| `TuiAltScreen` / `TuiAltScreenOptions` | `packages/tui/src/tui-alt-screen.ts` |
| `TuiMainScreen` | `packages/tui/src/tui-main-screen.ts` |
| `TuiMode` / `ViewportTUI` | `packages/tui/src/tui.ts` |
| `createInteractiveTui` / `switchTuiMode` / dock layout | `packages/coding-agent/src/modes/interactive/interactive-mode.ts` |
| `getOsc8LinkAtColumn` | `packages/tui/src/utils.ts` |
| 行为测 | `packages/tui/test/tui-alt-screen.test.ts` |
