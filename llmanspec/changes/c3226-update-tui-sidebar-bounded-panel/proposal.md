---
depends_on: []
title: "会话侧栏有界尺寸与固定区避让"
needs_specs_change: true
---

# 会话侧栏有界尺寸与固定区避让

## Why

现状的会话侧栏是整屏右列：开启时整屏按 `content_w` 渲染，侧栏带又由 `overlay_viewport` 画到所有可见行，输入框、队列条、待办栏、通知条、status、tray、footer 都被一起让宽或占用。侧栏只有 24/32/40 三档宽度、固定 3 行占位内容，没有高度上限，也没有通用的“有界面板”能力。

用户目标：侧栏支持任意限定尺寸与限位长度；在 app TUI 中右侧栏始终不遮盖输入框及其上方保留行，以有界侧框呈现。

后续 active / delayed changes 也需要可约束尺寸的侧板（会话图、workspace、loop/goal 状态、fork 摘要），继续用一次性手写布局会重复付账。

## 现状证据

- 侧栏布局与宽度档：`crates/xylitol-app-tui/src/layout/root/mod.rs:1017`、`:1004`、`:1025`。
- 整屏按侧栏让宽：`crates/xylitol-app-tui/src/layout/root/render.rs:380`。
- 侧栏带画到所有可见行：`crates/xylitol-app-tui/src/layout/root/render.rs:451`。
- 宿主让宽对账：`crates/xylitol-app-tui/src/host/session_ops.rs:11`；引擎侧 `extra_reserved_cols` 作用于整个组件渲染宽度：`crates/xylitol-tui/src/application_owned_runtime.rs:41`。
- 引擎 overlay 尺寸选项：`crates/xylitol-tui/src/tui/overlay.rs:43`（有 width/min_width/max_height/anchor/margin，无 max_width/min_height/clip rect）。
- overlay 合成按整屏定位：`crates/xylitol-tui/src/tui/overlay.rs:518`、`:605`。

## What Changes

### 引擎（crates/xylitol-tui）

- 提供通用的有界面板约束：可用矩形（available rect）+ 首选/最小/最大宽度 + 首选/最小/最大高度 + 锚点与内缩 + 溢出策略（裁剪 / 滚动）。
- 新增 `PanelRegion` + `PanelBounds` + `PanelPlacement::{DockLeft, DockRight, FloatLeft, FloatRight}`；扩展 `OverlayOptions` 的 `max_width`、`min_height`、`clip`（可用矩形）；面板只合成到 clip 内，不越过下缘固定区。
- 保持 `Component::render(width) -> Vec<String>` 契约；高度溢出用 `ScrollView`，内容不足按实际内容收口。
- 命中、滚轮、选区以面板实际矩形为准；面板外不穿透。
- 性能：面板合成只处理面板矩形内的行，不整屏克隆；不因面板开关触发 transcript 重排。

### 应用端（crates/xylitol-app-tui）

- 侧栏改为 transcript 区有界侧板：顶部从 transcript 顶开始，底部止于下缘固定区上缘（`transcript_rows`）；不覆盖输入框、队列条、待办栏、通知条、status、tray、footer。
- 顶部 loaded-resources 卡片不单独避让：侧栏从 transcript 顶开始，卡片随 transcript 让宽（B）或被悬浮盖住（A）。
- 宽度支持任意值（absolute / percent），24/32/40 作为预设快捷档；拖拽分隔条在 min/max 内调整任意宽度；宽度 clamp 到 min/max 与内容区最小宽度。
- 高度支持 max（行数 / 百分比）与 auto，默认 max = min(60% × transcript_rows, 24 行)、min = 3 行；内容超出内部滚动；内容不足按内容高度收口（“限位长度”）。
- 窄 / 矮降级：宽度或 transcript 高度不足时，先降级为 `FloatRight` 悬浮弹出框，仍不足则隐藏；切换动作静默。
- 宽度 / 高度经 client-local 存储全局持久化。
- 应用端采用 transcript 内驻列（方案 B）：transcript 按内容宽重排，下缘固定区保持全宽。
- 引擎同时支持有界悬浮（方案 A）与驻列让宽（方案 B）作为组件组合 / 参数组合；应用端不写两套特判。
- app TUI 保持固定产品命名（会话侧栏 / 左栏）；引擎组件与 placement 用通用命名；不重命名既有 `Panel` 或其他组件。
- 命中 / 滚轮 / 选区只作用于侧板矩形；侧板外不穿透。

### Specs（绑定分支后编辑）

- 更新 `app-tui-fixed-zone` r2904：把“最右整高”改为“transcript 区内有界侧板”，补充宽度 / 高度约束与降级语义。
- 更新 `package-tui-engine`：新增 `PanelRegion` / `PanelBounds` / `PanelPlacement` 与 overlay `clip` / `max_width` / `min_height` 的公开行为。
- 更新 `package-tui-pointer`：面板矩形的命中 / 选区裁剪写成引擎指针行为，与 app 侧命中口径同源。

## 布局模型候选

已决：应用端采用 B（transcript 内驻列）；引擎同时支持 A（有界悬浮）与 B（驻列让宽），以组件组合 / 参数组合暴露。以下保留 A/B/C/D 作为设计对照。

### A. 有界悬浮侧板（引擎候选，应用端不默认使用）

侧栏只占 transcript 区，悬浮在右上；transcript 保持全宽、不重排；下方固定区全宽且不被覆盖。宽度 / 高度可约束，内容超出滚动。性能最好：打开 / 关闭只重绘侧板矩形，不触发 transcript 重排。

```text
┌──────────────────────────────────────────────┐
│ ╭──────────────────────────────────────────╮ │
│ │ >_ XYlitol (version)                     │ │
│ ╰──────────────────────────────────────────╯ │
│ 对话正文 ........................... ┌──────┐ │
│ 对话正文 ........................... │侧栏  │ │
│ 对话正文 ........................... │内容  │ │
│                                    └──────┘ │
│ [队列条]                                     │
│ [待办栏]                                     │
│ [通知条]                                     │
│ ⬢ Working                                    │
│ ╭──────────────────────────────────────────╮ │
│ │ ❯ 输入框                                  │ │
│ ╰──────────────────────────────────────────╯ │
│ footer                                       │ │
└──────────────────────────────────────────────┘
```

### B. transcript 内驻列（应用端已选）

侧栏只占 transcript 区，但 transcript 按内容宽重排（不覆盖正文）；下方固定区全宽。打开 / 关闭会触发 transcript 重排；长会话有一次重排成本。

```text
┌──────────────────────────────────────────────┐
│ ╭──────────────────────────────────────────╮ │
│ │ >_ XYlitol (version)                     │ │
│ ╰──────────────────────────────────────────╯ │
│ 对话正文 ................... │ 侧栏          │ │
│ 对话正文 ................... │ 内容          │ │
│ 对话正文 ................... │               │ │
│                              │               │ │
│ [队列条]                                     │
│ [待办栏]                                     │
│ [通知条]                                     │
│ ⬢ Working                                    │
│ ╭──────────────────────────────────────────╮ │
│ │ ❯ 输入框                                  │ │
│ ╰──────────────────────────────────────────╯ │
│ footer                                       │ │
└──────────────────────────────────────────────┘
```

### C. 整列驻留 + 高度上限（现状延伸）

侧栏仍整高，输入框一起让宽；只增加高度上限与宽度约束。不满足“不遮盖输入框及其上方保留行”。

### D. 纯弹出 overlay

侧栏改为非捕获 overlay，悬浮在 transcript 右上；下方固定区全宽；宽度 / 高度可约束；不驻留。性能最好，但命中 / 焦点 / 交互要改走引擎 overlay。

## 引擎 API 形状（已决）

已决：引擎用通用命名 `PanelRegion` + `PanelBounds` + `PanelPlacement`；app TUI 保持固定产品命名（会话侧栏 / 左栏），不重命名既有 `Panel` 或其他组件。

- `PanelRegion`：包一个子组件，按 `PanelBounds`（首选 / 最小 / 最大宽高、锚点、内缩、溢出策略）渲染；高度溢出走 `ScrollView`，内容不足收口。
- `PanelPlacement::{DockLeft, DockRight, FloatLeft, FloatRight}`：
  - `DockRight` / `DockLeft`：app 在 transcript 内拼右侧 / 左侧面板（B），主区让宽、dock 行全宽。
  - `FloatRight` / `FloatLeft`：走 overlay + `clip`（A），面板只合成到 transcript 矩形内。
- 引擎扩展 `OverlayOptions`：补 `clip` / `max_width` / `min_height`。
- 应用端只选 placement，不写两套特判；横向拼接留在 app 一处 helper。

### 备选（未选）

- `BoundedPanel` + 引擎 `SidePanel`：引擎再提供横向组合组件；需要 dock-aware 渲染上下文，改动更大。
- 只扩 `OverlayOptions` + app helper：引擎改动最小；A/B 语义分散。
- 新增布局原语 `PanelSlot`：最通用；改动最大，超出本次侧栏需求。

## “限位长度”语义（已决）

已决：面板可见高度上限 + 内部滚动。`height = clamp(content_rows, min_height, min(max_height, transcript_rows))`；默认 `max_height = min(60% × transcript_rows, 24 行)`，`min_height = 3 行`；内容超出滚动，内容不足按实际高度收口。内容长度截断（B）与硬内容上限（C）未选。

## 尺寸模型（已决）

已决：通用约束（首选 / 最小 / 最大宽高 + available rect + 锚点）；24/32/40 仅作快捷档；拖拽分隔条在 min/max 内调整任意宽度。仅档位 + 高度上限（B）与配置固定值（C）未选。

## 降级策略（已决）

已决：窄 / 矮 → `FloatRight` 悬浮弹出框 → 仍不够则隐藏；切换动作静默。DockRight 沿用标准档 80 与内容区最小 40；FloatRight 在 `term_cols >= 26` 且 `transcript_rows >= 3` 时可用，面板宽 `clamp(preferred, 24, term_cols - 2)`，否则隐藏。直接隐藏（B）与压缩内容保持驻列（C）未选。

Fallback：若 `FloatRight` / Hidden 的实现复杂度超预期，可退回“压缩保持驻列”（仅 DockRight），不阻塞有界尺寸与固定区避让主目标；`FloatRight` 仍作为引擎能力保留，后续再接线。

## 性能与实现约束

- 应用端 B 会改 transcript 渲染宽度，导致 upper cache 失效并重排 transcript；这是打开 / 关闭侧栏的一次性成本，需要与悬浮 placement（A）区分。
- B 需要把 transcript 与下缘固定区拆成两个宽度渲染，避免固定区一起让宽。
- 引擎同时支持 A/B 时，悬浮 placement 只处理面板矩形内的行，驻列 placement 由宿主把内容宽度传给 transcript。
- 引擎 overlay 当前会克隆整屏 lines 再合成；有界面板应只处理面板矩形内的行。
- 命中 / 选区 / 滚轮必须按面板实际矩形裁剪，不能沿用“整列到屏幕底”的旧口径。

## 与 active / delayed changes 的关系

- `c3210-add-graph-ui`（delayed）：会话图视图需要大尺寸面板；有界面板 API 应可复用。
- `c3209-add-workspace-backend`（delayed）：workspace / branch 状态适合侧板。
- `c3207-add-loop-scheduler`（active）：下次触发时间 / 状态 / 上次结果需要展示面。
- `c3208-add-goal-tracking`（active）：目标进度 / 剩余迭代 / 证据需要展示面。
- `c3211-add-fork-context-reduction`（active）：fork 摘要 / 分支摘要适合侧板。
- `c3216-add-agent-profile-resume-ui`（active）：Choice 槽为主；有界面板可作为 resume 摘要的候选容器，但不是前置依赖。
- `c2911-add-tui-sidebar-followups`（archive 存根）：侧栏业务字段延后；本 change 是其后续。
- `c2875` / `c2905` / `c2910` / `c2913` / `c2918`：驻列、滚动分区、命名、档位降级的历史决策。

## Open Questions

无未决项；全部决策见下。

## 决策记录

- Q1 布局模型：应用端选 B（transcript 内驻列）；引擎同时支持 A（有界悬浮）与 B（驻列让宽），以组件组合 / 参数组合暴露，不把 A/B 写成两套 app 特判。
- Q2 引擎 API 形状：`PanelRegion` + `PanelBounds` + `PanelPlacement::{DockLeft, DockRight, FloatLeft, FloatRight}`；引擎补 `clip` / `max_width` / `min_height`，横向拼接留在 app 一处 helper。
- Q3 “限位长度”：可见高度上限 + 内部滚动；`height = clamp(content_rows, min_height, min(max_height, transcript_rows))`，不做内容长度截断或硬内容上限。
- Q4 宽度调整：档位循环 + 拖拽分隔条；宽度模型为通用约束（min/max + preferred），24/32/40 仅快捷档；持久化由 Q6 定为全局 key。
- Q5 高度默认值：百分比 + 绝对兜底；`max_height = min(60% × transcript_rows, 24 行)`，`min_height = 3 行`。
- Q6 持久化：宽度 / 高度存全局 key（client-local 存储），所有会话共享。
- Q7 降级策略：窄 / 矮先降级为 `FloatRight` 悬浮弹出框，仍不足则隐藏；切换动作静默。DockRight 沿用标准档 80 与内容区最小 40。
- Q7 fallback：若 `FloatRight` / Hidden 实现复杂，可退回“压缩保持驻列”（仅 DockRight），`FloatRight` 后置接线。
- Q8 降级状态：保留“想开”状态；空间恢复自动回到 DockRight。
- Q9 首批范围：DockRight + FloatRight + Hidden 全做；若 FloatRight 复杂度超预期，apply 阶段退回 fallback（仅 DockRight + 压缩驻列）。
- Q10 命名边界：引擎用通用命名 `PanelRegion` + `PanelBounds` + `PanelPlacement::{DockLeft, DockRight, FloatLeft, FloatRight}`；app TUI 保持固定产品命名（会话侧栏 / 左栏）；不重命名既有组件。本 change 只接右边栏，左栏后续复用同一引擎组件。
- Q11 FloatRight 阈值：`term_cols >= 26` 且 `transcript_rows >= 3` 时可用；面板宽 `clamp(preferred, 24, term_cols - 2)`；否则隐藏。
- Q12 顶部避让：只避开下缘固定区；侧栏占 transcript 行（含 loaded-resources 卡片行），卡片随 transcript 让宽（B）或被悬浮盖住（A）；不做顶部 inset。
- Q13 spec 范围：`app-tui-fixed-zone` + `package-tui-engine` + `package-tui-pointer` 三者都改。

## Further Notes

- 当前在 explore 阶段，未绑定分支，未编辑 `llmanspec/specs/**`；本 proposal 只记录方向与决策。
- 引擎 API / 状态机 / 命中与测试底稿：`research/engine-panel-api.md`。
- 全部决策已对齐（Q1–Q13）；当前停在 explore。等其他人处理完成后，补 `design.md` 推进到 designed，再进入 `llman-sdd-propose` / apply。
