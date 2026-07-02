# c381-simplify-tui-status-line — Tasks

> 简化状态行：去掉 Turn 轮数 / 模型名，只留 spinner + 活动标签。模型名永不填充（ModelSelect 事件 agent 层从不发出）。

## 0. 规划工件

- [x] proposal.md + design.md + specs/app-tui/spec.toon（modify tui53）
- [x] `llman sdd validate c381 --strict` 通过

## 1. 简化（status_line.rs + app.rs）

- [x] `StatusSegments` 删 center/right 字段，留 streaming + left_label
- [x] `StatusLine` widget 删三段布局，改 spinner + 单标签
- [x] `TuiApp` 删 turn_index/model_name 字段
- [x] `handle_xy_event` TurnStart/ModelSelect 回退 no-op
- [x] `status_segments()` 不再产 center/right

## 2. 校验

- [x] StatusLine harness 修订（idle/streaming/tool/spinner 推进）
- [x] `cargo test --features tui --lib` 全绿
- [x] `cargo fmt` + `clippy --features tui` 干净
- [x] 同步 tui/AGENTS.md + 归档
