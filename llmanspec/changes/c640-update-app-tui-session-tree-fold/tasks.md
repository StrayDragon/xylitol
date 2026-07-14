# Tasks — c640-update-app-tui-session-tree-fold

- [ ] 1. Delta 校验：`LLMANSPEC_BASE_REF=origin/main llman sdd validate c640-update-app-tui-session-tree-fold --no-interactive`
- [ ] 2. 树槽转发 `ctrl+left|alt+left|ctrl+right|alt+right`（或等价 matches）到 `tree.handle_input`
- [ ] 3. harness：fold 隐藏后代 + ⊞；unfold 恢复；确认裸 ←→ 仍翻页
- [ ] 4. 同步 keybindings / session-tree 文案（若仍写「未接线」）
- [ ] 5. `just fmt` + 相关 test / clippy 绿
