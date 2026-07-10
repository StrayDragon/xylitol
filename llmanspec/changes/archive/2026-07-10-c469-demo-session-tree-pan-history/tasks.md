# Tasks — c469-demo-session-tree-pan-history

> 先实现后补规格：下列任务在提案时已完成（见 `ab54a95`）。

- [x] 1. Delta：`package-tui-tree-selector` pts10 水平 pan；`app-tui-session-tree` ast3/ast4 活树与 travel 回复链
- [x] 2. 包：`render_horizontal_viewport` + 单测 `horizontal_pan_keeps_deep_selected_label_visible`
- [x] 3. demo：活 `session_tree`、submit/tool/assistant 挂树、`travel_path_with_replies`、travel 后 Ready
- [x] 4. harness：submit 挂树、travel 含回复、tool 挂树
- [x] 5. 回写 `session-tree-vs-pi.md` / `session-tree.md`
- [x] 6. `cargo test -p xylitol-tui --lib tree_selector`；`cargo test -p xylitol-tui --test agent_demo_test session_tree`；`llman sdd validate c469-demo-session-tree-pan-history --strict --no-interactive`
