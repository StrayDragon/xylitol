# Tasks — c645-update-app-tui-session-tree-fork

- [x] 1. Delta 校验：`LLMANSPEC_BASE_REF=origin/main llman sdd validate c645-update-app-tui-session-tree-fork --no-interactive`
- [x] 2. 修 `SessionManager` fork：`get_branch` + 重链 parent_id；父文件只读；user Before / 非 user At（对齐 pi）
- [x] 3. 树槽 `shift+f` → pending；effects：`fork` + `switch_session` + 关树/刷新；user 预填 Before、非 user At 不预填
- [x] 4. 测：兄弟不泄漏、父不变、user-before / assistant-at；harness h19/h20；session BDD 分叉仍为 stub（路径语义由 unit 覆盖）
- [x] 5. 同步 keybindings / session-tree（产品 fork ≠ demo ast5；注明存储合约）
- [x] 6. `just fmt` + 相关 test / clippy 绿
