# Tasks — c615-update-app-tui-live-session-tree

- [x] 0.1 确认 c610 已归档（`depends_on`）
- [x] 1.1 `SessionTreeNode` → `TreeNode` 映射（kind + 纯正文 label）
- [x] 1.2 开树：`session_tree(MessageHistory)` 填充 Tree 槽；移除生产假树依赖
- [x] 1.3 Enter：`travel_session_tree` → 关树 / 预填 / 刷新 transcript
- [x] 2.1 harness + ScriptedDriver：活树打开、user/non-user Enter
- [x] 3.1 更新 `session-tree.md` / vs-pi / keybindings / AGENTS / playground
- [x] 4.1 `llman sdd validate c615-update-app-tui-live-session-tree --strict --no-interactive`
- [x] 4.2 `cargo test -p xylitol --lib app::tui::tests -- --test-threads=1`
