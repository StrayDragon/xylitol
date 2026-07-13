# Tasks — c626-add-app-tui-playground-lint

- [x] 1. Delta specs 校验：`llman sdd validate c626-add-app-tui-playground-lint --strict --no-interactive`
- [x] 2. 添加 `src/app/tui/design/fixtures/{session-tree.filter,models.open}.yaml`
- [x] 3. playground：`data-design-fixture` + 清除 Next-wave 区 change-id 噪音；确认 `.rev * { color: inherit }`
- [x] 4. 实现 `scripts/check_tui_design_playground.py`（`--check`）并本地跑绿
- [x] 5. 更新 `design/AGENTS.md` 与 playground README（lint 指针）
- [x] 6. `just check-scripts-wired` + `python3 scripts/check_tui_design_playground.py --check` 通过
