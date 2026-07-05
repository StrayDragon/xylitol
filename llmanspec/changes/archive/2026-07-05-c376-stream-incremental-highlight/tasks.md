# c376 — Tasks

- [x] 1.1 app.rs：TuiApp 加 `committed_count: usize` + `last_full_render: Vec<Line>`（缓存全量渲染结果供 mutable 用）。handle_xy_event TextDelta 改全量重渲 + 行差量 commit。
- [x] 1.2 mutable_line.rs：MutableLine 改接收 `&[Line<'static>]`（渲染行）而非 `&str`。tail.rs 适配。
- [x] 1.3 pending_tail 改返回 `&[Line]`（尾部渲染行）。
- [x] 1.4 TurnEnd commit 剩余 `committed_count..` 行。
- [x] 2.1 测试 + 回归 + qa + 归档。
