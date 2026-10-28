# Tasks

- [ ] 1. **中间件**（plugin）：移除 `loaded` 短路，每次 `before_model` 读取最新 `plan_mode`；存储不可用时静默降级。测：execute → plan → execute 三次调用的注入矩阵。
- [ ] 2. **默认配置**（config）：`LoopExampleConfig::default().plan_mode = true`；`configs/example.yaml` 与 schema 同步。测：默认注册 `plan-mode`。
- [ ] 3. **specs**（spec）：确认 `agent-plan` / `agent-runtime` 对「下次模型调用生效」的表述；无行为合约新增。测：`llman-sdd validate --all --strict`。
- [ ] 4. **门禁**（repo）：`just qa` 全绿。测：`just qa`。
