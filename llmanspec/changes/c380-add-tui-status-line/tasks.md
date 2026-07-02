# c380-add-tui-status-line — Tasks

> TUI 加固定状态行（三段式：spinner+活动标签 / Turn 轮数 / 模型名），始终占位，位于 mutable 行下方、输入面板上方。接线现有未渲染的 Spinner widget。三段布局为可扩展骨架（对标 vim airline）。

## 0. 规划工件

- [x] proposal.md + design.md + specs/app-tui/spec.toon（add tui53，含三段扩展性护栏）
- [x] `llman sdd validate c380 --strict` 通过（tasks unchecked 为实现期警告）

## 1. TuiApp 状态字段 + 事件（app.rs）

- [x] 加 `turn_index: Option<u32>` + `model_name: Option<String>` 字段
- [x] `handle_xy_event`：TurnStart → 记 turn_index；ModelSelect → 记 model_name
- [x] 新增 `status_segments() -> StatusSegments`（数据驱动三段内容）
- [x] 删 app.rs 本地 `SPINNER`，改 `use ...spinner::SPINNER`（单一来源）

## 2. StatusLine widget（components/status_line.rs）

- [x] 新增 StatusLine：三段布局（left 左对齐 / center 居中 / right 右对齐），消费 StatusSegments
- [x] streaming 时接线 Spinner widget（固定 left 段首位，用 app.spinner_idx()）
- [x] components/mod.rs 导出 StatusLine

## 3. Tail 三段布局（tail.rs + terminal.rs）

- [x] Tail render：MutableLine（顶）→ StatusLine（中，1 行）→ BottomPanel（底）
- [x] TAIL_HEIGHT 5 → 6
- [x] input_cursor_position 仍正确（panel 在底，cursor 在 panel 内）

## 4. 测试

- [x] StatusLine harness：idle Ready+model / streaming spinner+Working+Turn+model / tool running / spinner 推进 / idle 无 spinner
- [x] 更新既有 Tail + render.rs harness 行号断言（mutable 20→19 / 1→0，status 新增行）
- [x] 顺带消除既存的 Spinner dead-code 警告（接线后不再 dead）
- [x] `cargo test --features tui --lib` 78 全过
- [x] `just test` 564 全过（1 既存 leak 无关）
- [x] `cargo clippy --features tui` + fmt 干净
- [x] 同步 tui/AGENTS.md（文件布局 + Out of scope）
