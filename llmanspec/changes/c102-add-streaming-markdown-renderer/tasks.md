# c102-add-streaming-markdown-renderer Tasks

- [x] 在 `ChatComponent` 中添加 `StreamingRenderer` 字段（`Option` 包裹）
- [x] 修改 `append_assistant_delta` 使用 `streaming.push(delta)` 代替直接 content 拼接
- [x] 在 `render` 方法中集成 `streaming.tick()` 获取增量行
- [x] 修改 `finish_streaming` 调用 `streaming.finish()` 并缓存最终输出
- [x] 处理 `set_width` 变更时通知 `StreamingRenderer` 更新宽度
- [x] 确保 `raw_output=true` 路径不使用 StreamingRenderer
- [x] 添加单元测试：验证 debounce 行为和 dual-phase 输出
- [x] 更新 insta 快照（如渲染输出有细微差异）
- [x] `just fmt && just lint && just test`
- [x] `llman sdd validate c102-add-streaming-markdown-renderer --strict --no-interactive`
