# Tasks — c1160-update-app-tui-paste-collapse

- [x] 1. Delta + design + tasks 齐；`LLMANSPEC_BASE_REF=main llman sdd validate c1160-update-app-tui-paste-collapse --strict --no-interactive`
- [x] 2. 包：`Editor::get_expanded_text` 按完整 marker 替换；单测折叠/展开/短粘贴
- [x] 3. 产品：`UiRoot::editor_text`（或发送路径）改用 `get_expanded_text`；harness 提交全文
- [x] 4. 若有意不做原子 marker 分段，在 `packages/xylitol-tui/PI_DELTAS.md` 记一行
- [x] 5. `just fmt` + 相关包/产品测试；validate --strict
