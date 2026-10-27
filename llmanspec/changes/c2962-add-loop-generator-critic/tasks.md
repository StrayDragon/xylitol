# Tasks

- [ ] 1. **spec**（spec）：`agent-runtime` 新增 `r2944`「generator-critic 中间件」，并在 `r2925` 补注册场景。测：`llman sdd validate c2962-add-loop-generator-critic --strict`。
- [ ] 2. **配置**（plugin）：`LoopExampleConfig` 增 `generator_critic: GeneratorCriticConfig`；缺省关闭、`max_rounds` 缺省 1。
- [ ] 3. **中间件**（plugin）：新增 `GeneratorCriticMiddleware`（order 170，`before_model` / `after_model` / `decide_continue`）；状态机、verdict 解析、有界停止、无 verdict 重试一次；插件在启用时注册并一并注销。测：中间件单测（approve/revise/无 verdict/上限/tool_calls 不触发/禁用基线）+ 注册/卸载单测。
- [ ] 4. **配置产物**（config）：`just gen-config-schema`；`configs/example.yaml` 补 `generator_critic`。测：config schema 漂移单测。
- [ ] 5. **BDD**（tests）：新增注册场景 given/then 与 binding。测：`cargo test --all-features --test bdd test_lm_generator_critic_registers`。
- [ ] 6. **文档**（docs）：`src/AGENTS.md` 补「critic 是 core 调度的模型调用，不是插件内调用」。测：`just qa-doc-check`。
- [ ] 7. **门禁**（repo）：`just qa` 全绿。测：`just qa`。
