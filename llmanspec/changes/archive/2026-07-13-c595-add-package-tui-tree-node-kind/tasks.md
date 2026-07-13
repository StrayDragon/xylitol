# Tasks — c595-add-package-tui-tree-node-kind

- [x] 1.1 包：`TreeNode.kind` + `with_kind`；主题 `kind_prefix`；渲染与搜索纳入 kind
- [x] 1.2 包单测：kind 前缀可见；搜索匹配 kind；无 kind 时行为与旧版一致
- [x] 2.1 demo：样例/活树设 kind，label 去 role 前缀；filter 改看 kind
- [x] 2.2 harness：树行含着色 kind 前缀（或可辨识 `user:` 由主题输出）
- [x] 3.1 更新 `session-tree.md` / `session-tree-vs-pi.md`（废除「预渲染进 label」）
- [x] 3.2 playground `index.html` 树槽：kind 用 `fg-user`/`fg-assistant`/`fg-tool` + 纯正文
- [x] 4.1 `llman sdd validate c595-add-package-tui-tree-node-kind --strict --no-interactive`
- [x] 4.2 `just test-tui`（或至少 tree_selector + agent_demo 相关测）
