# Tasks — c505-refactor-provider-adapter-path

- [ ] 1. 画出当前 Completions/Responses/Anthropic 装配图（简短，可放 design 或 PR 说明）
- [ ] 2. 去掉 `OpenAIProvider: XyModel` 对外路径；Completions 仅作 `LlmAdapter`
- [ ] 3. 统一经 `AdapterXyModel`（或等价单一外壳）注入 `dyn XyModel`
- [ ] 4. 更新 factory / 测试（含 fake）
- [ ] 5. `rg async_openai src/agent src/domain` 为零
- [ ] 6. `llman sdd validate c505-refactor-provider-adapter-path --strict --no-interactive`
- [ ] 7. `cargo test` provider/adapter 相关 + `just lint`
