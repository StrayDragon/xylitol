# Design — c425-port-editor-core

## 决策 1：VisualLine 数据模型 — struct vs tuple

| 方案 | 做法 | 结论 |
|---|---|---|
| **A. 独立 struct** | `VisualLine { logical_line: usize, start_col: usize, len: usize }` | ✅ 采用 |
| B. tuple `(usize, usize, usize)` | 三元素元组 | ❌ 拒绝 |

### 理由

- `build_visual_line_map` 返回 `Vec<VisualLine>`，在 `move_to_visual_line` 中需要反复访问三个字段。具名字段比 `.0`/`.1`/`.2` 更可读。
- 不需要导出到 lib.rs（内部类型），但将来 c430 autocomplete 集成可能需要检查 VL 注入点——具名 struct 容易查找。

## 决策 2：Sticky column — preferred_visual_col 存储列偏移 (visual) vs 字节偏移 (logical)

| 方案 | 做法 | 结论 |
|---|---|---|
| **A. 视觉列偏移** | `preferred_visual_col: Option<usize>` 存相对于 VL.start_col 的偏移 | ✅ 采用 |
| B. 逻辑列 | 存绝对 `cursor_col` | ❌ 拒绝 |

### 理由

- **完全对齐 pi**：pi 的 `preferredVisualCol` 是 VL 内的偏移量（:1384），不是绝对列号。如果存绝对列号，当 VL 分配因 resize 改变时 sticky 行为错误。
- `snapped_from_cursor_col` 存绝对列号（对齐 pi :1413），因为它是"返回逻辑位"的还原坐标。

## 决策 3：PasteBurst 时间注入 — Instant 参数 vs Box<dyn Clock>

| 方案 | 做法 | 结论 |
|---|---|---|
| **A. Editor 持有 Clock trait object 并传 Instant** | `editor.clock.now()` 然后在调用处传 `now: Instant` 给 paste_burst | ✅ 采用 |
| B. PasteBurst 持有 Clock | paste_burst 内部调 `clock.now()` | ❌ 拒绝 |

### 理由

- **c415 已验证**：PasteBurst 方法接受 `Instant` 参数，不持有 clock。编辑器持有 `Box<dyn Clock>` 并在每个调用点传 `self.clock.now()`。测试时 MockClock 确定性控制时间。
- Editor 未来可能还有其他时间驱动行为，持有 Clock 是合理的长期投资。

### 后果

- `Editor::new()` 增加 `clock: Box<dyn Clock>` 参数（或 `impl Clock + 'static`）
- 测试用 `MockClock`，生产用 `SystemClock`
- 测试中 `insert_ch` 前后推进 `MockClock` 的时间

## 决策 4：History 重构策略 — 增量修改 vs 重写

| 方案 | 做法 | 结论 |
|---|---|---|
| **A. 增量修改** | 保留 `hist_nav` 但改名/改语义，新加 `exit_history_browsing` 入口 | ✅ 采用 |
| B. 全量重写 | 删除历史相关代码重写 | ❌ 拒绝 |

### 理由

- xy 的 `hist_nav` / `history_draft` / `add_to_history` 语义大部正确，只需：
  - `hist_nav` → `navigate_history(direction: isize)`（改名 + 改 `setTextInternal` 调用）
  - `setTextInternal(text, cursor_placement: CursorPlacement)` 新方法
  - `exit_history_browsing()` 在 12 个编辑动作入口调用
- 全量重写风险大且不必要

## 决策 5：pageScroll 页大小 — 终端行数依赖

| 方案 | 做法 | 结论 |
|---|---|---|
| **A. 渲染时从 TUI 注入或构造参数** | `EditorOptions { terminal_rows: Option<usize> }` | ✅ 采用 |
| B. 硬编码常量 | `const PAGE_SIZE: usize = 5` | ❌ 拒绝 |

### 后果

- `EditorOptions` 增加 `terminal_rows: Option<usize>`（默认 None → fallback 到 24）
- `render()` 的 `max_vis` 计算改用 `terminal_rows`：
  - 旧：`5.max(self.state.lines.len().min(10))`
  - 新：`max(5, terminal_rows * 30 / 100)`（对齐 pi）

## 非目标

- 不集成 autocomplete（c430 范围）
- 不移植 `segmentWithMarkers`（paste-marker-aware 分段，可延后）
- 不修改 Editor 的 pub API 破坏性（`on_submit`/`on_change`/`disable_submit` 保留）
- 不依赖 TUI 引用（Editor 不持有 `&TUI`，通过构造参数传递 terminal_rows）
