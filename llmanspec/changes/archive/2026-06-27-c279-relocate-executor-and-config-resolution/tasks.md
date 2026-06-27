# c279-relocate-executor-and-config-resolution Tasks

## Core ports & vocabulary

- [x] 1. 在 `core::ports` 定义 `BashResult` 与 `BashExecutor` port
- [x] 2. 新建 `src/core/bash.rs`，提供纯函数 `parse_bang_prefix`
- [x] 3. 在 `core::ports` 定义 `SecretResolver` port
- [x] 4. 把 `AgentsFile` 从 `infra::resource::loader` 移到 `core::resource_types`
- [x] 5. 在 `core::ports` 定义 `ResourceLoader` port
- [x] 6. 在 `core::ports` 定义 `TrustStore` port

## Infra implementations

- [x] 7. 新建 `src/infra/bash_exec/mod.rs`，迁移原 `agent/runtime/bash.rs` 实现并提供 `InfraBashExecutor`
- [x] 8. 新建/更新 `infra/config` 提供 `InfraSecretResolver`
- [x] 9. `DefaultResourceLoader` impl `ResourceLoader`
- [x] 10. `TrustManager` impl `TrustStore`

## Agent-side decoupling

- [x] 11. 删除 `src/agent/runtime/bash.rs` 与 `src/agent/session/bash_exec.rs`
- [x] 12. `AgentSession` 注入 `Arc<dyn BashExecutor>`，迁移 `execute_bash` / `abort_bash` / `process_prompt`
- [x] 13. `ModelRegistry` 注入 `Arc<dyn SecretResolver>`，更新 `has_resolved_auth` / `resolve_provider_headers`
- [x] 14. `agent/prompt/system.rs` 改用 `&dyn ResourceLoader`
- [x] 15. `AgentSession::save_trust_decision` 改用 `&dyn TrustStore`
- [x] 16. `Agent::with_ports` / `Agent::new` 传播 `bash_executor` 参数

## Composition roots

- [x] 17. `interactive/cli/mod.rs` 构造 `InfraBashExecutor`、`InfraSecretResolver` 并注入；更新 `ModelRegistry::new` 调用
- [x] 18. `interactive/rpc.rs` 同上
- [x] 19. `server/runtime.rs` 同上（server 注入默认 executor 即可）

## Tests & guards

- [x] 20. 更新所有 `ModelRegistry::new()` 调用点（测试、facade、示例等）
- [x] 21. 从 `src/tests.rs::AGENT_INFRA_ALLOWLIST` 删除 c279 的 7 项条目
- [x] 22. 运行 `cargo build` 并修复编译错误
- [x] 23. 运行 `cargo test --lib` / `cargo nextest run --profile ci`
- [x] 24. 运行 `cargo test --test bdd -- --test-threads=1`
- [x] 25. 运行 `cargo clippy` 并修复新增 warning
- [x] 26. 运行 `cargo test --lib arch_guard` 确认白名单只剩 c278 12 项
- [x] 27. 运行 `llman sdd validate c279-relocate-executor-and-config-resolution --strict --no-interactive`
