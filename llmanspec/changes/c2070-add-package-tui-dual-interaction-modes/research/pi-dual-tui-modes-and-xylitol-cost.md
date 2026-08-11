# Research: Pi 双 TUI 模式证据 × xylitol-tui 跟进代价 × ratatui

> 日期：2026-08-11
> Change：`c2070-add-package-tui-dual-interaction-modes`
> 一手：`../pi/packages/tui`、`../pi/packages/coding-agent`；仓内 `packages/xylitol-tui`（`PI_DELTAS.md` / `NOTICE`）。

## 0. 结论

| 问题 | 答案 |
|---|---|
| Pi 两种模式在哪？ | **库**：`TuiMainScreen`（`mode="regular"`）vs `TuiAltScreen`（`mode="fullscreen"`）。**产品**：`createInteractiveTui` 按 `tuiMode === "fullscreen"` 择一；设置 `tuiMode` 默认 `regular`。 |
| 哪侧是应用内选区？ | **仅 AltScreen**：进 alt-buffer + 开 mouse tracking + `handleSelectionMouseEvent` / 复制 / ScrollView。MainScreen **无** mouse/selection 实现。 |
| xylitol-tui 今日对齐哪侧？ | **MainScreen / regular / inline 差分**（`previous_viewport_top` + 终端 scrollback）。无 AltScreen 渲染器、无 app selection。 |
| 跟进代价？ | **大**：不是「加个 flag」，而是第二套（或可切换的）视口+鼠标+选区子系统；量级约 **数周级** 引擎工作 + 产品接线，接近再实现一版 Pi fullscreen 鼠标栈的 Rust 版。 |
| ratatui 有帮助吗？ | **对 Mode B 原型/算法有参考价值；不宜整包替换 xylitol-tui。** ratatui = 立即模式全屏画布，与今日 inline 差分引擎不同族；引入等于第三条栈。可选：借 `tui-panel-select` 等**算法**，或独立 lab 用 ratatui 探 Mode B UX，再 port 回 xylitol-tui。 |

## 1. Pi 代码证据（路径均相对 `../pi`）

### 1.1 产品择一（coding-agent）

```343:351:packages/coding-agent/src/modes/interactive/interactive-mode.ts
export function createInteractiveTui(options: InteractiveTuiOptions): TuiMainScreen | TuiAltScreen {
	const terminal = options.terminal ?? new ProcessTerminal();
	if (options.tuiMode === "fullscreen") {
		return new TuiAltScreen(terminal, options.showHardwareCursor, options.logDirectory, {
			openUrl: openBrowser,
			onRightClickPaste: options.onRightClickPaste,
		});
	}
	return new TuiMainScreen(terminal, options.showHardwareCursor, options.logDirectory);
}
```

- 设置：`SettingsManager.getTuiMode()` / `setTuiMode("fullscreen"|"regular")`（测试钉默认 `regular`）。
- 文档：`packages/coding-agent/docs/keybindings.md` — fullscreen 下拖选复制、滚轮、OSC8 点击等。

### 1.2 MainScreen = emulator-owned / inline

```56:58:packages/tui/src/tui-main-screen.ts
/** TUI implementation that renders into the terminal's main screen and scrollback. */
export class TuiMainScreen extends TuiBase implements TUI {
	readonly mode = "regular" as const;
```

- 持有 `previousLines` / `previousViewportTop`（与 xylitol-tui 差分同族）。
- **源码无** mouse tracking / selection 字段或 handler（相对 AltScreen）。

### 1.3 AltScreen = application-owned

```44:50:packages/tui/src/tui-alt-screen.ts
const ENTER_ALT_SCREEN = "\x1b[?1049h";
...
const ENABLE_BUTTON_MOTION_MOUSE = "\x1b[?1000h\x1b[?1002h\x1b[?1004h\x1b[?1006h";
const ENABLE_ALL_MOTION_MOUSE = "\x1b[?1000h\x1b[?1002h\x1b[?1003h\x1b[?1004h\x1b[?1006h";
```

```130:130:packages/tui/src/tui-alt-screen.ts
	readonly mode = "fullscreen" as const;
```

启动写入（节选逻辑）：`ENTER_ALT_SCREEN` +（若 `mouseEnabled`）mouse CSI + clear；多路复用器用 button-motion（1002）而非 all-motion（1003）以降噪。

选区：`selectionAnchor` / `selectionFocus`、`handleSelectionMouseEvent`、`copySelectionToClipboard`；布局用 `ScrollView`（应用视口，非终端 scrollback）。

库出口：`packages/tui/src/index.ts` 同时 `export` 两者 —— **双实现，会话择一**（可运行中切换，属换栈）。

### 1.4 与 oneof 的对应

| Pi 名 | xylitol c2070 称呼 | 选区 | 滚动 |
|---|---|---|---|
| `regular` / `TuiMainScreen` | Mode A emulator-owned | 终端原生 | 终端 scrollback |
| `fullscreen` / `TuiAltScreen` | Mode B application-owned | 应用内 | ScrollView / app wheel |

## 2. xylitol-tui 现状与跟进代价

### 2.1 今日

- Fork 自 pi-tui **inline/差分**路径（`NOTICE`、`PI_DELTAS.md`）；产品 host 驱动。
- `c2020`：opt-in `EnableMouseCapture`，**无** app selection / AltScreen 渲染器。
- 组件树有 SelectList 等「列表选中」，**不是** transcript 文本拖选。

### 2.2 若跟进 Pi「双模式」

| 工作包 | 内容 | 量级（粗） |
|---|---|---|
| B1 视口 | alt-buffer 或自管 viewport；历史不再只靠模拟器 scrollback | 大 |
| B2 鼠标 | 常开/模式内 tracking；wheel→scroll；Moved 过滤（已有纪律） | 中 |
| B3 选区 | anchor/focus、高亮重绘、词/行、OSC52/剪贴板 | 大 |
| B4 布局 | 固定 editor/footer + 可滚 transcript（Pi fullscreen 布局） | 中～大 |
| B5 产品闸 | `tuiMode` 设置、切换 teardown、与 ath25/差分脏区契约 | 中 |
| B6 折叠消费 | Mode B 上才有「自然直接点」（starline 族） | 挂 c2040 等 |

**Mode A 保持**：跟进不等于废弃 inline；Pi 也是默认 regular。
**不能**用「只 port `handleSelectionMouseEvent`」在 Mode A 上得到 fullscreen 体验。

### 2.3 与 c1505 的关系

Mode A 的 entry viewport **切片**仍可能有价值（少 flatten）；Mode B 的「可见行」定义会变。故 c1505 与本族一并 delay、软挂 c2070。

## 3. ratatui 帮不帮忙？

| 用法 | 评价 |
|---|---|
| **替换** xylitol-tui 为 ratatui | **不建议**。产品已押注差分 inline、host 驱动、大量组件/harness；ratatui 是另一套立即模式 + 通常全屏。成本 ≈ 重写应用面。 |
| **Mode B 用 ratatui、Mode A 保留 xylitol-tui** | 双栈运维/主题/键位灾难；仅当接受长期两套 UI 才考虑。 |
| **借用生态算法** | **有帮助**：如 `tui-panel-select`（面板选区几何/复制）、`ratatui-interact`（click region）——作参考实现，port 进 xylitol-tui Mode B。 |
| **Lab 原型** | 可用 ratatui 快速试 Mode B UX，验证后再在 xylitol-tui 落地（对齐 Pi：库内自研 AltScreen，未用 React/Ink）。 |

Crush（Go/Bubble Tea）与 ratatui 同属「全屏自管」族，可作行为参照，不是 crate 依赖。

## 4. 产品时序与 Mode B MUST（2026-08-11 拍板）

| 阶段 | 做什么 |
|---|---|
| 现在 | **delay c2070**；继续打磨 **inline（Mode A）** 产品 TUI |
| 之后 | 调研实现双模式；Mode B = **alt-screen 族** |
| 再后 | 产品 **可能**切 Mode B；`cascade/` 折叠/点击按依赖升格 |

Mode B **默认 MUST**（补齐今日 inline 靠终端原生选区能做的事）：

1. 未修饰拖选 + 高亮
2. **跨页 / 越界续选**（拖到视口边缘自动滚）
3. **松手自动复制**（默认开）
4. 与折叠 hit 共存：标记列消费 click，其余走选区

（Pi 对照：`TuiAltScreen` 的 selection + edge auto-scroll + `copySelectionToClipboard`。）

## 5. 建议（给 c2070 升格时）

1. 文档与旗标先钉 **Mode A 默认 / Mode B opt-in**（对齐 Pi）。
2. Mode B **在 xylitol-tui 内自建**（port Pi AltScreen 概念 + 可选参考 tui-panel-select），**不**引入 ratatui 为运行时依赖。
3. 折叠点击族（c2040…）声明：**自然直接点 = Mode B**；Mode A 最多 Alt-hold/模式/Shift-选。
