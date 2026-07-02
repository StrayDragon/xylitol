# c381 Design — 状态行简化

## 决策

c380 三段式状态行真终端测试后简化：去掉 center（Turn 轮数）和 right（模型名），只留 spinner + 单活动标签。

理由：
- **模型名永不填充**：`XyEvent::ModelSelect` 在 agent 层从不发出（全仓库无 yield/emit，只有消费方），`model_name` 字段恒为 None。
- **Turn 轮数非必需**：用户反馈不需要。
- 用户明确：只需 spinner + Working。

## 改动

- `StatusSegments`：删 `center`/`right` 字段，留 `streaming` + `left_label`。
- `StatusLine` widget：删三段 Layout，改 spinner + 单 Span 标签（左对齐）。
- `TuiApp`：删 `turn_index`/`model_name` 字段；`handle_xy_event` 的 TurnStart/ModelSelect 回退为 no-op；`status_segments()` 不再产 center/right。
- TAIL_HEIGHT 不变（status 仍 1 行）。

## 测试

StatusLine harness 修订：idle=Ready、streaming=spinner+Working、tool=工具名、spinner 推进。去掉 center/right 相关断言。
