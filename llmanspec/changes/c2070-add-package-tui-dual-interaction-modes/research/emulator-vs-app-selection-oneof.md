# Research: 终端原生选区 vs 应用内选区（术语与 oneof）

> 日期：2026-08-11
> Change：`c2070-add-package-tui-dual-interaction-modes`（原散落在 c2040 调研，2026-08-11 集中至此）
> 动机：澄清「直觉点击折叠是否必须上 app 选区」；统一专业词；对照 pi-tui 是否整体是 app 选区架构。
> 相关：同目录 `native-selection-vs-click-fold.md`、`pi-starline-click-expand-vs-rust.md`、`alt-hold-capture-and-scrollback-hit.md`。

## 0. 结论

| 问题 | 答案 |
|---|---|
| 直觉「无修饰点折叠 + 拖选像普通终端」要什么？ | 协议层上，**未修饰左键**不能同时既归终端选区又归应用 click。要「点像 GUI、选也像 GUI」，通常上 **应用内选区**（并把 wheel/scroll 一并应用化）。 |
| 两个模式的专业叫法？ | 见 §1。核心对立：**emulator-owned（终端/仿真器选区）** vs **application-owned（应用内选区）**；开关侧常称 **mouse tracking / mouse capture（grabbed）**。 |
| 是否「无法调和的 oneof」？ | **对「同一时刻、未修饰左键的归属」是 oneof（互斥）**。产品可用 **模式切换 / 修饰键旁路** 在时间上切换两侧，但不能在同一手势上两边同时拥有。 |
| pi-tui 整体是 app 选区架构吗？ | **否。** 它是 **双渲染器 oneof**：`TuiMainScreen`（inline）≈ 终端选区；`TuiAltScreen`（fullscreen）≈ 应用选区 + tracking。starline 点展开只挂在后者。 |

## 1. 术语对照（建议仓内统一用）

| 中文（建议） | 英文（专业/文档常用） | 指什么 | → 代码标识符（c2070） |
|---|---|---|---|
| **终端原生选区** / **仿真器选区** | **emulator-owned selection**、**terminal-native selection**、**host selection** | 选区由终端仿真器画和高亮；复制走终端/OSC 52/主选区；应用**不**解释拖选几何 | `InteractionMode::Inline`；不挂 `ApplicationOwnedRuntime`；无应用内 transcript 选区 |
| **应用内选区** | **application-owned selection**、**app-managed / client-side selection** | 应用收鼠标、自绘高亮、自算行列、自复制（OSC52/剪贴板工具） | `InteractionMode::ApplicationOwned` + `ApplicationOwnedRuntime` / `selection`；入口 `ApplicationOwnedTui` |
| **鼠标上报 / 捕获** | **mouse tracking**、**mouse capture**、**xterm mouse reporting**（DECSET 1000/1002/1003…） | 终端把鼠标事件编码发给应用；开着时应用处于 Kitty 所称 **grabbed** | `enable_mouse_capture` / `begin_application_owned_session`（AO 路径一并开） |
| **未捕获 / 已捕获** | **ungrabbed** / **grabbed**（Kitty `mouse_map`） | 未开 tracking ≈ 终端处理 click/拖选；开了 ≈ 应用优先 | Inline 默认 ungrabbed；AO 会话 grabbed |
| **选区旁路修饰键** | **selection-override modifiers**（foot 等） | tracking 开着时，按住 Shift（等）把该次手势**临时还给**终端选区 | （产品未承诺；库不假装兼得） |
| **应用视口滚动** | **application-managed scroll** / **viewport scroll** | 内容滚在应用状态里（ScrollView），不是模拟器 scrollback | AO 内 `ScrollView`；`set_dock_rows` / `dock_rows_hint`；`editor_screen_origin` |
| **终端历史上滚** | **terminal scrollback** / **emulator scrollback** | 历史行在仿真器缓冲；应用通常**点不到** | Inline 差分写主缓冲；AO 退出 `finish_application_owned` + `set_append_session_to_main_scrollback_on_exit` |
| **Inline 交互栈** | **inline / main-screen stack**（≈ Pi `TuiMainScreen` / regular） | 主屏差分 + 终端选区取向；一次会话主模式之一 | `InteractionMode::Inline` · `finish_inline` |
| **ApplicationOwned 交互栈** | **application-owned stack**（≈ Pi `TuiAltScreen` / fullscreen） | 常经 alt-buffer；应用视口 + 应用内选区 + dock 排除 | `InteractionMode::ApplicationOwned` · `begin`/`end`/`finish_application_owned` |

仓内口语可继续说「app 选区 / 原生选区」；写入 proposal/spec / AGENTS 时优先用表中英文与 **Inline / ApplicationOwned**。禁止新代码用 `mode_a` / `mode_b`（见 `packages/xylitol-tui/AGENTS.md` §8）。

**易混**：

- **Mouse capture ≠ 应用内选区。** Capture 只解决「事件给谁」；选区还要自己实现高亮/拖/复制，否则用户只剩 Shift-选或什么都选不了。
- **Alt-hold 临时 capture ≠ 应用内选区。** 只是短时把归属切到应用做 click；松手回到终端选区。
- **Ratatui / Crush / Pi fullscreen** 多半整包在 **grabbed +（常有）app selection + app scroll** 一侧。

## 2. 直觉体验 ↔ 归属

```text
未修饰左键 Down
  ├─ ungrabbed（无 tracking）→ 终端开始拖选；应用收不到 → 不能点折叠
  └─ grabbed（有 tracking）  → 应用收到事件
        ├─ 实现了 app selection → 可：点折叠 / 拖选高亮 / 复制（starline / Crush 族）
        └─ 未实现 selection   → 可点折叠，但未修饰拖选「坏了」（除非 Shift 旁路）
```

因此：

- 「像 starline 那样点开，拖选也自然」→ **grabbed + application-owned selection**（通常还要 app scroll）。
- 「保持现在 xylitol 拖选/上滚手感」→ **ungrabbed** 为主；点击只能靠修饰键/模式临时 grabbed。

## 3. 做应用内选区通常还要带什么（你的担心属实）

不是「只加一个 Selection 结构体」就结束。典型连带：

| 机制 | 为何几乎必带 |
|---|---|
| 选区状态（anchor/focus、粒度 char/word/line） | 拖选几何 |
| 命中与行缓冲坐标 | 屏幕 (x,y) → 内容行列；差分引擎要钉 viewport 算术 |
| 高亮重绘 | 选区变化 → dirty 行；与 ath25 / fingerprint 交互 |
| 复制（OSC52 / wl-copy 等） | 否则选了无法带出 |
| **Wheel → app scroll** | grabbed 后轮不再滚终端历史；要「上滚看旧内容」就得自管视口 |
| （可选）自动滚、双击词、三击行、右键粘贴 | Pi AltScreen / Crush 已有的「完整感」 |

对 **inline + 终端 scrollback** 产品：上全套 app 选区 ≈ **换交互模型**（接近再实现一版 Pi fullscreen 鼠标栈），确实是 **TUI 大改 / 独立大切片**，不宜塞进「点个三角」的 c2040 最小切片。

## 4. pi-tui：不是单一架构，是模式 oneof

| 渲染器 | 产品入口 | 选区模型 | 鼠标 |
|---|---|---|---|
| `TuiMainScreen` | 默认 interactive（非 fullscreen） | **emulator-owned** | 无 tracking（源码无 selection mouse） |
| `TuiAltScreen` | `--tui-mode fullscreen` | **application-owned** | 开 tracking；自带 selection / wheel / URL click 等 |

→ **pi-tui 库同时提供两侧实现**；**一次会话选一个渲染器**（可切换，但是换栈，不是同一帧混用未修饰归属）。
starline 的 click-expand **只**建立在 AltScreen 那一侧之上。

xylitol-tui 今日定位更接近 **MainScreen / inline** 一侧；要「像 fullscreen 点折叠」是在 **换到另一侧模型**，不是在同一侧打补丁。

## 5. 「无法调和的 oneof」精确说法

| 命题 | 真假 |
|---|---|
| 「未修饰左键同时：终端拖选 + 应用点折叠」 | **假**（协议互斥） |
| 「产品永远只能二选一、永不提供另一侧」 | **假**——可 **模式切换**（Pi main↔fullscreen）、**修饰键旁路**（Shift 选 / Alt-hold 点）、或 **分阶段** 后上 app 选区 |
| 「inline 原生手感 + starline 级直接点」可低成本兼得 | **假**——兼得要付 app 选区（+ 多半 app scroll）成本 |

一句话：**归属是 oneof；产品策略可以是「可切换的 oneof」，不是「同时成立的 both」。**

## 6. 对 xylitol 的产品含义（调研建议，不拍板）

1. 调研有价值：先钉 **我们站在 oneof 的哪一侧**，再谈 c2040 范围。
2. 若默认侧继续是 **emulator-owned + inline**：直接点不是本波 MUST；用 G/F/B′ 之一，并写清代价。
3. 若要对齐 starline 直觉：**另开**「应用内选区（± fullscreen/app scroll）」change，视为交互模型升级，而非折叠三角的附属项。
4. 仓内文档避免说「开了 mouse 就有选区」；应说「开了 tracking（grabbed）；选区另论 emulator vs application」。
