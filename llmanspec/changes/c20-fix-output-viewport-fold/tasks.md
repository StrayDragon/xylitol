## 测试 seam（复用既有 harness，不新发明）

1. **包层纯函数**：`render_expandable_output` / `ExpandableOutput`（`packages/xylitol-tui/src/components/expandable_output.rs` 模块内单测）。
2. **app 层 headless 交互**：`UiRoot::click_fold_at` / `toggle_fold_target` + `render_scrollback` + `FoldHitTable`（`src/app/tui/tests.rs`、`src/app/tui/widgets/scrollback/tests.rs` 既有 att20/att22/att29–att32 测试族）。
3. **bridge 层**：`UiModel` 直播 / rebuild 幂等测试（`src/app/tui/bridge/tests.rs`、`session_tree.rs`）。

## Tasks

### T1 包层：expanded 折叠提示行（已完成）

- `ExpandableOutputOptions` 新增 `fold_hint: String`（默认 `"ctrl+o to fold"`）。
- `render_expandable_output`：expanded 且视觉行数 > `max_preview_lines` 时块尾追加 dim `... (expanded, {fold_hint})`；否则不追加。
- 单测：展开超限有提示行、展开未超限无提示行、fold 文案来自选项、既有 streaming/zero-width 用例不回归。
- 撑 spec：`package-tui-expandable-output` peo1 / peo3（本 change 内已 landing）。

### T2 bridge：Bash 稳定 id（已完成）

- `UiEntry::Bash` 加 `id: String`；直播 `begin_bash_block` 分配序号 id；rebuild（`session_tree.rs`）填 `bashId`。
- 修齐既有构造点 / 测试夹具。

### T3 app 层：按块视口翻转 + fold 行命中（已完成）

- `blocked-by: T1, T2`
- `ScrollbackFold.output_overrides` + `output_effective(id)` + `toggle_output(id)` + `clear_output_overrides()`；`FoldTarget::OutputViewport(String)`。
- `paint_tool_block` / `paint_diff_block` / `paint_bash_block` 用 `output_effective(id)` 决定 viewport；`push_expandable_with_viewport_hit` 对 collapsed hint 行与 expanded fold 行注册带 id 命中。
- cache fingerprint 三处改为哈希 per-block effective 值（ath25：单块点击只重绘该块）。
- `root/mod.rs` `toggle_fold_target` 按块翻转；`slot_input.rs` Ctrl+O 翻全局默认 + 清覆盖（不整表失效缓存，仅 bump 受影响 fingerprint）。
- headless 测试：点 A 块 hint 只翻 A；B 块不动；点 A 的 fold 行折回 A；Ctrl+O 全局翻转并清覆盖；硬截断块点击不展开（att16 不回归）。

### T4 门禁收口（已完成）

- `blocked-by: T3`
- `just fmt` / `just lint` / `just test` / `just test-tui` 全绿；`llman sdd validate <id> --strict --no-interactive` 全绿。
