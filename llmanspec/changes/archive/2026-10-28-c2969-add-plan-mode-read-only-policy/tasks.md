# Tasks

- [x] 1. **specs**（spec）：`agent-plan` 增 `plan-mode-read-only`（allowlist、拒绝原因、不可执行）；`agent-tools` 记录计划模式闸不得执行被拒工具、不得绕过 core 权限；`agent-runtime` 记录多 `before_tool` 闸顺序。测：`llman-sdd validate --all --strict`。
- [x] 2. **闸实现**（plugin）：`PlanModeMiddleware` 增 `before_tool`；默认允许 read/grep/find/ls/ask/plan_*/todo_*，拒绝 write/edit/bash 与未知工具；拒绝时改写 args 并 `stopped`。测：允许/拒绝矩阵。
- [x] 3. **配置**（config）：`plan.read_only`（默认 true）与 `plan.allowed_tools`；schema/example 同步。测：schema 漂移与默认值。
- [x] 4. **模式切换**（plugin）：批准或 `/plan off` 后闸立即失效；缓存模式缺失时按 fail-closed 规则处理。测：切换后 write 恢复。
- [x] 5. **BDD**（tests）：计划模式拒绝 edit；切回 execute 后允许 edit。测：`cargo test --all-features --test bdd`。
- [x] 6. **门禁**（repo）：`just qa` 全绿。测：`just qa`。
