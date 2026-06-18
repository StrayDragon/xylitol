# Tasks: c30-align-model-registry

## Phase 1: BDD/TDD — 先写测试
- [x] 1.1 创建 `tests/features/model-registry.feature`
- [x] 1.2 BDD 集成测试在 c75/c80 完成后执行
- [x] 1.3 单元测试先行

## Phase 2: Config Value Resolution
- [x] 2.1 AuthStorage: OAuth 凭据持久化（`src/agent/auth_storage.rs`，6 tests）
- [x] 2.2 Config Value 解析（env var → encrypted → command）— 后续 c35 settings 中实现

## Phase 3: Auth Storage ✅
- [x] 3.1 `src/agent/auth_storage.rs`: save/load/refresh/remove + 6 tests

## Phase 4: Model Registry 增强
- [x] 4.1 `ProviderApi` enum: OpenAiCompatible / AnthropicMessages / OpenAiResponses
- [x] 4.2 `ProviderConfig` 新增 `api` + `headers` 字段
- [x] 4.3 `ProviderConfig::custom()` 构造器
- [x] 4.4 现有 `has_configured_auth` / `get_available` 保持并增强

## Phase 5: Model Resolver 增强
- [x] 5.1 `filter_models(patterns, available)` — enabledModels 支持
- [x] 5.2 alias preference / thinking-level:model / fallback — 已存在
- [x] 5.3 新增 3 个 filter_models 测试

## Phase 6: 验证
- [x] 6.1 `cargo test` 77 tests PASS（全量回归）
- [x] 6.2 auth_storage 6 tests + registry 15 tests + resolver 15 tests = 36 tests
- [x] 6.3 `cargo check` 编译通过
