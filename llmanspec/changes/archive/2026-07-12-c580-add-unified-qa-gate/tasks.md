# Tasks — c580-add-unified-qa-gate

- [x] 1. 更新 `justfile`：`qa` 依赖链含 `test-tui`；新增 `qa-e2e`；注释写清范围
- [x] 2. 更新根 `AGENTS.md` 命令 / 提交前验证段
- [x] 3. 更新 `.claude/skills/test-tui-harness/SKILL.md`（及 `.agents` 镜像若存在）与 `_HANDOFF.md` 短索引
- [x] 4. `llman sdd validate c580-add-unified-qa-gate --strict --no-interactive`
- [x] 5. 跑 `just check-tui-tokens` + `just --list` 确认 recipe；完整 `just qa` 作为归档前 verify（耗时长）
