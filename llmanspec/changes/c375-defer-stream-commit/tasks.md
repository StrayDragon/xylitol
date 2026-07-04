# c375-defer-stream-commit — Tasks

> 决策见 design.md：延迟到 TurnEnd 整段 commit；移除 stream_buf/thinking_buf，用 finalized/thinking_finalized 字符串累积；TAIL_HEIGHT 动态=终端/2。

## 阶段 1：TuiApp 数据流改造

- [x] 1.1 app.rs：移除 `stream_buf`/`thinking_buf` 字段，改用 `finalized: String` + `thinking_finalized: String`。更新 `start_stream`/`end_stream` 的 clear 逻辑。
- [x] 1.2 `handle_xy_event`：TextDelta 只 `finalized.push_str(text)` 返回空 Vec；ThinkingDelta 同理累积 `thinking_finalized`；thinking→text 转换在 TextDelta 首次到达时做。
- [x] 1.3 `handle_xy_event` TurnEnd：整段 `finalized`→`AssistantText`、`thinking_finalized`→`ThinkingText`，`std::mem::take` 清空。
- [x] 1.4 `pending_tail`：改返回 `&finalized`（text 阶段）/ `&thinking_finalized`（thinking 阶段）。
- [x] 1.5 `end_stream`：XyDone 兜底——若有残留 finalized 未 commit（stream 异常结束），mod.rs 的 XyDone 处理先 commit 残留再 end_stream。

## 阶段 2：TAIL_HEIGHT 动态化

- [x] 2.1 terminal.rs `enter()`：读终端高度，`tail_height = (h/2).max(6)`，传入 `Viewport::Inline(tail_height)`。

## 阶段 3：测试更新

- [x] 3.1 更新 app.rs 测试：TextDelta 返回空 Vec（不再逐行 commit）；TurnEnd 返回单个整段 AssistantText；残留 finalize 在 XyDone commit。
- [x] 3.2 新增 TestBackend 测试：TextDelta 累积代码块 → TurnEnd → render_markdown 识别围栏 → 高亮 span（非 default style）。
- [x] 3.3 回归 commit_harness CJK + transcript_line + mutable_line + render.rs 测试。

## 阶段 4：校验 + 归档

- [ ] 4.1 `just qa`（all-features）+ `arch_guard` 4 + `llman sdd validate --strict` 通过。
- [ ] 4.2 归档 c375。

## 反降级护栏

- [x] 代码块在 finalize 后有高亮（TurnEnd commit 的 AssistantText 经 render_markdown 识别围栏，spec tui75）。
- [x] 流式期间 mutable 区显示累积文本（逐字效果保留）。
- [x] 多轮工具调用每段独立 commit。
