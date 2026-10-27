# Tasks

- [ ] 1. **spec**（spec）：`agent-runtime` 新增 `r2940`「工具可见性收窄」；`r2925` 补「工具收窄启用后注册工具中间件」场景。测：`llman sdd validate c2959-add-loop-minimal-tool-set --strict`。
- [ ] 2. **配置**（plugin）：`LoopExampleConfig` 增 `tool_exposure: ToolExposureConfig`；默认关闭、规则为空。
- [ ] 3. **中间件**（plugin）：新增 `ToolExposureMiddleware`（`before_model`，order 50，allow/deny/read_only，计划工具始终可见）；插件在启用时注册并一并注销。测：中间件单测 + 插件注册/卸载单测。
- [ ] 4. **配置产物**（config）：`just gen-config-schema`；`configs/example.yaml` 注释补 `tool_exposure`。测：config schema 漂移单测。
- [ ] 5. **BDD**（tests）：新增注册场景的 given/then 与 binding。测：`cargo test --all-features --test bdd test_lm_tool_exposure_registers`。
- [ ] 6. **门禁**（repo）：`just qa` 全绿。测：`just qa`。
