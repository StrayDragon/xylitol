# Tasks — c04-add-agent-profile-foundation

- [x] 拆分 `src/agent/provider/mod.rs` → `mod.rs` + `fake.rs`
- [x] 新增 `src/agent/profile.rs` (ResolvedProfile)
- [x] 新增 `AgentProfile`, `AgentsConfig` 到 `src/infra/config/types.rs`
- [x] 新增 `AppConfig::resolve_model()`, `resolve_profile()`, `resolve_default_profile()`
- [x] 重构 `AgentLoop::new()` 接受 `ResolvedProfile`
- [x] 新增 `ToolRegistry::filtered()` 方法
- [x] 更新 `cli/mod.rs`: `build_resolved_profile()` 替代 `build_model_config()`
- [x] 更新 `print.rs`: `run_print()` 接受 `&ResolvedProfile`
- [x] Run `just fmt && just lint && just test`
