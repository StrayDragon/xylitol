# Tasks

- [ ] 1. **spec**（spec）：`agent-runtime` 新增 `r2943`「self-heal 中间件」，并在 `r2925` 补注册场景。测：`llman sdd validate c2961-add-loop-self-heal --strict`。
- [ ] 2. **配置**（plugin）：`LoopExampleConfig` 增 `self_heal: SelfHealConfig`；缺省关闭、`max_attempts` 缺省 3。
- [ ] 3. **中间件**（plugin）：新增 `SelfHealMiddleware`（order 210，`after_tool` + `decide_continue`）；命中签名时记录 pending 并追加 `<self_heal>` 指引；同一 tool 成功清除；cap 内 Continue、到 cap Stop；插件在启用时注册并一并注销。测：中间件单测（命中/不命中、成功清除、cap、非确定性失败）+ 注册/卸载单测。
- [ ] 4. **配置产物**（config）：`just gen-config-schema`；`configs/example.yaml` 补 `self_heal`。测：config schema 漂移单测。
- [ ] 5. **BDD**（tests）：新增注册场景 given/then 与 binding。测：`cargo test --all-features --test bdd test_lm_self_heal_registers`。
- [ ] 6. **文档**（docs）：`src/AGENTS.md` 补「自愈只在确定性失败上启用且有独立上限」。测：`just qa-doc-check`。
- [ ] 7. **门禁**（repo）：`just qa` 全绿。测：`just qa`。
