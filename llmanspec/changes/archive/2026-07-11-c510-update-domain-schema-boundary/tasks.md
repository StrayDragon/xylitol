# Tasks — c510-update-domain-schema-boundary

- [x] 1. 列出 `src/domain` 中所有 JsonSchema derive 位点
- [x] 2. 为仍需 schema 的配置结构确认 infra 侧宿主（已有则去重，缺则加 DTO + From）
- [x] 3. 移除 domain 的 schemars 依赖用法；确认 domain 可无 schemars 编译
- [x] 4. 跑配置加载 / settings 相关测试
- [x] 5. `llman sdd validate c510-update-domain-schema-boundary --strict --no-interactive`
- [x] 6. `just lint` + 相关 `cargo test`
