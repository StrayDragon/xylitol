# Tasks — c510-update-domain-schema-boundary

- [ ] 1. 列出 `src/domain` 中所有 JsonSchema derive 位点
- [ ] 2. 为仍需 schema 的配置结构确认 infra 侧宿主（已有则去重，缺则加 DTO + From）
- [ ] 3. 移除 domain 的 schemars 依赖用法；确认 domain 可无 schemars 编译
- [ ] 4. 跑配置加载 / settings 相关测试
- [ ] 5. `llman sdd validate c510-update-domain-schema-boundary --strict --no-interactive`
- [ ] 6. `just lint` + 相关 `cargo test`
