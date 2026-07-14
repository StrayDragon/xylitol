# Tasks — c635-update-app-tui-session-tree-filter

- [x] 1. Delta 校验：`LLMANSPEC_BASE_REF=origin/main llman sdd validate c635-update-app-tui-session-tree-filter --no-interactive`
- [ ] 2. `session_tree` mapper：`label`→`annotation`；bookkeeping → `kind=meta`
- [ ] 3. 产品 FilterMode + `include_node` / `status_suffix`；挂载树时应用 default
- [ ] 4. 树开键：Ctrl+D set default；Ctrl+T/U/L/A toggle；Ctrl+O cycle；优先于 thinking/viewport
- [ ] 5. harness：no-tools 藏 tool；user-only；labeled；toggle 回 default；键入搜索 + Esc 清搜索；cycle 后缀
- [x] 6. 同步 `session-tree.md` / `session-tree-vs-pi.md` / `keybindings.md` 与 design 决议（若文案仍写 demo set）
- [ ] 7. `just fmt` + 相关 test / clippy 绿
