# Research: pi-starline 点击展开 × Rust 生态对等（c2070）

> 日期：2026-08-11
> Change：`c2070`（自 c2040 调研迁入）
> 动机：仍想支持**无修饰直接点击**折叠；对照 `../pi-starline` 的 `clickToExpandTools`，问 Rust / xylitol 是否有对等路径。
> 一手：`/home/l8ng/Projects/__straydragon__/pi-starline`、`../pi/packages/tui`、`../crush`；同目录 `native-selection-vs-click-fold.md` / `alt-hold-capture-and-scrollback-hit.md`。

## 0. 结论（可引用）

| 问题 | 答案 |
|---|---|
| starline「点一下就展开」是怎么做到的？ | **不是**在保留终端原生拖选的前提下「偷」点击。它挂在 Pi **fullscreen / `TuiAltScreen`** 上：该模式**已经**开 mouse tracking，并有**应用内选区**；starline 只在 `handleSelectionMouseEvent` 上拦截「落在 expand hint 行」的左键 Press，调 `setExpanded`，其余 press **放行给 Pi 选区**。 |
| 这和 xylitol 今日模型对等吗？ | **不对等。** xylitol-tui ≈ Pi 的 **main / inline** 路径（`TuiMainScreen`）：**无** mouse、**无** app selection。Pi 自己的 inline 主屏同样没有这套能力；starline 文档也写明已从「自管 compositor」迁到 **patch Pi 0.84 renderer**。 |
| Rust 有没有「装一个 crate = starline 点击展开」？ | **没有**针对 inline 差分引擎的 drop-in。Ratatui 生态有的是 **alt-screen + capture 已开** 前提下的 hit-test / 面板选区（如 `ratatui-interact`、`tui-panel-select`、`edtui`），语义上更接近 **Pi AltScreen / Crush**，不是「原生选区 + 免费 click」。 |
| 若坚持「直接点」在 xylitol？ | 必须先接受 **方案 C 族**：开 capture + **应用自管选区**（及通常还有 app scroll），再在选区路径上加「标记列 / hint 行 hit → toggle」——那才是 starline 的真实对等。仅点折叠、选区仍用 Shift（方案 B）是**半对等**（有 click，无 app 选区手感）。 |

## 1. starline 实际做了什么（一手）

### 1.1 功能名与配置

- 配置键：`mouse.clickToExpandTools`（默认 true；总闸 `mouse.enabled`）。
- 文档：`pi-starline/docs/configuration.md` § Mouse features。
- 实现：`extensions/starline/mouse/tool-box.ts` + `index.ts` 对 `handleSelectionMouseEvent` 的 patch；能力表见 `capabilities.ts`（feature `clickToExpandTools` 依赖 `handleSelectionMouseEvent` / `hasOverlay` / `requestRender`）。

### 1.2 命中规则（不是点三角）

- **目标行**：Pi 渲染的 `(ctrl+o to expand|collapse)` **hint 行**（跟 keybinding 文案走），不是边框、也不是折叠三角字形。
- **为何不是 border**：Pi 自带 tool box 无框；框来自可选扩展 `pi-toolbox`。
- **解析**：strip ANSI → 正则匹配 hint → 在 component tree 里找带 `setExpanded` 的祖先 → toggle 那一块。
- **未命中 hint**：`call through` → Pi 正常开始选区。

### 1.3 前置条件（常被忽略）

```text
Pi --tui-mode fullscreen
  → TuiAltScreen
  → ENABLE_*_MOUSE（1000/1002[/1003]/1004/1006）
  → handleSelectionMouseEvent / copySelectionToClipboard / app scroll…
Starline installMouse
  → patch 上述方法
```

对照：

| | Pi `TuiMainScreen`（inline） | Pi `TuiAltScreen`（fullscreen） | xylitol-tui（今日） |
|---|---|---|---|
| 缓冲 | 主屏 + 终端 scrollback | alt-screen 自管视口 | inline + 终端 scrollback |
| mouse tracking | **无**（源码无 mouse/selection） | **有** | opt-in `c2020`，产品默认关 |
| 拖选 | **终端原生** | **应用内选区** | **终端原生**（capture 关时） |
| starline click expand | **不可用**（无 patch 点） | **可用** | 无对等原型可 patch |

→ starline 的「直接点」买的是：**已经把鼠标从终端拿走，用应用选区补回来**；点击展开只是同一条事件管线上的一个分支。

Starline 自己也写过：曾尝试「frame-free selection」等更野的路，**砍掉了**；今日路径明确依赖 Pi 的 selection mouse API。

## 2. Crush（Go）对照

`../crush`（Charm / Bubble Tea 栈）同样在 **应用已收鼠标** 的前提下做 click expand：

- `AssistantMessageItem.HandleMouseClick`：点 thinking box → 走 `ToggleExpanded`。
- UI 文案：`[click or space to expand]`。
- 另有 selection drag / highlight（`BeginSelectionDrag` 等）——仍是 **app-owned**，不是终端原生选区。

与 starline 同族：**fullscreen/app 画布 + capture → 选区与 click 同源**。

## 3. Rust 生态「对等」地图

| 能力切片 | TS/Go 参照 | Rust 大致对等 | 与 xylitol inline 的距离 |
|---|---|---|---|
| Mouse 事件扇入 | crossterm / pi SGR parse | **已有** `c2020` / crossterm `InputEvent::Mouse` | 近 |
| Hit-test / click region | starline `hit-test` + component tree；crush `HandleMouseClick` | `ratatui-interact`（`ClickRegion`）；自建表（c2040 意向） | 中：需产品自己建 fold hit 表 |
| 应用内文本选区 + 复制 | Pi `TuiAltScreen` selection；Crush selection | `tui-panel-select`、`edtui` mouse select、自研 | **远**：差分 inline 上叠选区高亮 / 拖 / OSC52，且与 ath25 重绘纠缠 |
| 应用 scroll（可点「任意」历史） | Pi alt-screen ScrollView；Crush list | ratatui 自管 viewport | **远**：与「终端原生历史上滚」产品定调冲突 |
| 「仅拦截 hint/标记、其余选区」 | starline patch selection press | 无独立 crate；须在**自有** selection 管线上分支 | 仅当先有 app selection 才「像 starline」 |

**没有**发现：在 **不** Enable mouse capture、或 capture 开着但选区仍完全交给终端的前提下，Rust crate 能稳定实现「无修饰直接点折叠」。协议层不允许按列拆分归属（见前篇）。

## 4. 对 xylitol「仍想直接点击」的含义

把「直接点」拆成两档，避免和 starline 品牌对齐时偷换前提：

| 档 | UX | 必须具备 | 成本 | 是否 starline 对等 |
|---|---|---|---|---|
| **B′ 半对等** | 无修饰点 ▸/▾；选区改用 Shift（或模式键） | capture + hit + per-block toggle | **低**（c2040 原 MVP） | 有 click，**无** app 选区 |
| **C 全对等** | 无修饰点折叠 **且** 拖选像 Pi fullscreen / Crush | capture + **应用选区** +（通常）app wheel/scroll | **高** | **是** starline 所在族 |
| **G Alt-hold** | 按住 Alt 才点得到 | 动态 capture + Alt alone | 中 | 否；保原生选区为主 |
| **F 模式** | 开「折叠点击」模式后再无修饰点 | 模式闸 + capture | 低 | 否 |

若产品文案写「像 starline / Pi 一样点开」——诚实对齐的是 **C（或至少先上 fullscreen/alt-buffer 路线）**，不是在 inline + 原生选区上「再找一个 Rust 库」。

若产品坚持 **inline + 终端 scrollback 为默认**（今日定调）：

- **直接点**只能选 **B′**（接受选区/滚副作用）或 **先做 C 大切片**；
- starline 的 tip（hint 行命中、未命中放行选区）在 B′ 里变成「未命中 → 无选区 / 忽略」或「未命中仍吞事件」——没有「放行给终端选区」这回事，除非关 capture。

## 5. 建议（调研层，不自动开 tasks）

1. **不要**假设「移植 starline 插件思路」能在现有 inline 上白嫖直接点；要对等就先对齐 **Pi fullscreen 的鼠标所有权模型**。
2. 若本波仍要「无修饰点折叠」且不接受开 C：钉 **B′**，产品写清 Shift-选 / 滚轮归属；hit 目标用 **标记列**（比 starline 的 hint 文案更稳，也更贴 Web 三角）。
3. 若「直接点 + 自然拖选」都是硬需求：单独立案 **应用内选区**（可参考 `tui-panel-select` 算法 + Pi/Crush 行为），**blocks** 或并入后续 change；c2040 可先落 hit/覆盖表接口，选区后接同一 Mouse 管线。
4. Rust 侧可复用的是 **算法/模式**（hit 表、press 消费、选区几何），不是 npm 插件式 patch 宿主原型。

## 6. 证据索引

| 主张 | 证据 |
|---|---|
| starline patch selection press | `pi-starline/.../mouse/index.ts` 头注释；`capabilities.ts` `clickToExpandTools` |
| 命中 hint 行 | `tool-box.ts`；`docs/configuration.md` |
| 仅 fullscreen 有 mouse | `pi/.../interactive-mode.ts` `createInteractiveTui`；`tui-alt-screen.ts` ENABLE_MOUSE；`tui-main-screen.ts` 无 mouse |
| Pi 文档：fullscreen 拖选复制 | `pi/packages/coding-agent/docs/keybindings.md` altScreen 段 |
| Crush click expand | `crush/.../assistant.go` `HandleMouseClick` |
| xylitol fork 自 pi-tui inline 系 | `packages/xylitol-tui/NOTICE` |
