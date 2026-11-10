---
depends_on: []
title: "Fix TUI engine pointer, selection and focus state"
---

# TUI 引擎输入 / 选区 / 焦点状态修复

## Why

xylitol-tui 引擎的三个高风险状态机（静态扫描，实施前复核）：

- **指针管线 8 字段**（E2，pointer.rs:292，Lead 已复核 :440-540）：capture_latch 只能由 Up 清——丢失或乱序 Up 后永久吞点击；set_phase 外部改写不清 pending_click；dragged 与 down_cell 生命周期不同步。
- **选区双实现**（E3/E19，selection.rs:98 与 editor/selection.rs:8）：同一状态机两份实现已分叉（editor 版缺 dock clamp 与自动滚动）；dragging 与 anchor/focus 无耦合不变量。
- **overlay 焦点 4 写 1 修**（E1，overlay.rs:83-524）：focused_index/focused_overlay_id/overlay_focus_restore 三字段约 8 可达态，存态与读态两份真值，Blocked 一致性靠 dispatch 二次修补。
- **ChoicePrompt 越界编码 Review**（E4）：tab == questions.len() 编码 Review 页，states[self.tab] 靠 render 提前 return 保住，任一移动即 panic。
- **overlay 可见性**（E7）：hidden 与 focus_order 无约束，可聚焦到 hidden overlay；last_screen_rect 旧矩形命中。
- **gutter 拖拽仅 Up 清**（E24）：与 E2 同构的丢失 Up 残留。

## What Changes

- 指针管线状态显式化（Idle/Down/Captured/PendingClick），capture_latch 改为可由 Down 重入或超时回收；选区两实现合一（引擎层单一 controller，editor 特化以配置表达）。
- overlay 焦点收敛单一焦点状态对象，存储态与投影态合一，dispatch 二次修补删除。
- ChoicePrompt Review 态显式枚举，消灭越界下标编码。

## Capabilities

- package-tui-*（引擎行为契约，具体 cap 以 context 定位）。

## Impact

- 范围：crates/xylitol-tui/src/{pointer,selection}.rs、tui/overlay.rs、components/choice_prompt.rs、components/editor/selection.rs、application_owned_runtime.rs。
- 验证：丢 Up 事件注入的指针测试；选区行为等价性对照（合并前先冻结两实现行为快照）；overlay 焦点转移矩阵。

## Open Questions

1. 选区合一落点：引擎层 SelectionController 吸收 editor 特化（推荐）还是 editor 保留薄包装？
2. 指针状态机是否顺带统一 chrome_click 判定与 pending_click 生命周期？
