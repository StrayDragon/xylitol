# Tasks: c240-consolidate-architecture

## T1: 清除 pi 文档引用（30 个文件）
- [x] `src/agent/session.rs` — 移除 "Aligns with pi's AgentSession class"
- [x] `src/agent/loop.rs` — 移除 "Aligns with pi's agent-loop.ts"
- [x] `src/agent/prompt.rs` — 移除 "Aligns with pi's buildSystemPrompt()"
- [x] `src/agent/queue.rs` — 移除 "Aligns with pi's AgentSession steer/followUp"
- [x] `src/agent/templates.rs` — 移除 "Aligns with pi's prompt-templates.ts"
- [x] `src/agent/output_guard.rs` — 移除 "Aligns with pi's output-guard.ts"
- [x] `src/agent/config_value.rs` — 移除 "Aligns with pi's resolve-config-value.ts"
- [x] `src/agent/bash_executor.rs` — 移除 "Aligns with pi's core/bash-executor.ts"
- [x] `src/agent/commands.rs` — 移除 "Aligns with pi's slash-commands.ts"
- [x] `src/agent/retry.rs` — 移除 "Aligns with pi's _isRetryableError"
- [x] `src/agent/http_dispatcher.rs` — 移除 "Aligns with pi's http-dispatcher.ts"
- [x] `src/agent/tools/mutation.rs` — 移除 "Aligns with pi's file-mutation-queue.ts"
- [x] `src/agent/tools/accumulator.rs` — 移除 "Aligns with pi's output-accumulator.ts"
- [x] `src/agent/model/registry.rs` — 移除 "Aligns with pi's model-registry.ts"
- [x] `src/agent/model/resolver.rs` — 移除 "Aligns with pi's model-resolver.ts"
- [x] `src/agent/trust/store.rs` — 移除 "Aligns with pi's trust-manager.ts"
- [x] `src/agent/trust/resolve.rs` — 移除 "Aligns with pi's project-trust.ts"
- [x] `src/agent/auth/guidance.rs` — 移除 "Aligns with pi's auth-guidance.ts"
- [x] `src/agent/provider/attribution.rs` — 移除 "Aligns with pi's provider-attribution.ts"
- [x] `src/infra/timing.rs` — 移除 "Aligns with pi's timings.ts"
- [x] `src/infra/source_info.rs` — 移除 "Aligns with pi's source-info.ts"
- [x] `src/infra/resource/mod.rs` — 移除 "Aligns with pi's resource-loader.ts"
- [x] `src/infra/resource/loader.rs` — 移除 "Aligns with pi's resource-loader.ts"
- [x] `src/infra/session/mod.rs` — 移除 "Aligns with pi's session-manager.ts"
- [x] `src/infra/session/cwd.rs` — 移除 "Aligns with pi's session-cwd.ts"
- [x] `src/infra/settings/storage.rs` — 移除 "Aligns with pi's FileSettingsStorage"
- [x] `src/infra/settings/manager.rs` — 移除 "Aligns with pi's SettingsManager"
- [x] `src/interface/rpc.rs` — 移除 "Aligns with pi's modes/rpc/"
- [x] 校验：`rg "Aligns with pi" src/` → zero matches

## T2: ToolManager 内联回 AgentSession
- [x] 将 `active_tools: Vec<String>` 和 `registry: ToolRegistry` 从 ToolManager 移到 AgentSession
- [x] 将 ToolManager 的 4 个方法（active_tools、set_active_tools、registry）直接在 AgentSession 上实现
- [x] 更新 `session.rs` 中引用 `self.tool_manager` 的代码为直接访问 fields
- [x] 移除 `src/agent/tool_manager.rs`
- [x] 从 `agent/mod.rs` 移除 `pub mod tool_manager`
- [x] 校验：`cargo test --lib` → 503 passed

## T3: 创建宏系统评估文档
- [x] 创建 `docs/architecture/macro-registration.md`，分析：
  - `#[tool]` 过程宏：编译期注册 vs 当前 ToolRegistry::register 运行时
  - `#[command]` 宏：静态命令表 vs 当前 Vec<SlashCommandInfo>
  - `#[provider]` 宏：ModelKind 匹配生成 vs 当前 ModelConfigExt::build
  - ROI 分析与实施建议
- [x] 校验：文档完整性

## T4: ModelManager 简化
- [x] 经代码审计：`model_manager.set_thinking_level()` 已是纯字段设置（无隐式持久化回调）
- [x] 经代码审计：`build_current_model()` 被 session 和 loop 两处调用，内联会引入重复代码 → 保持现状
- [x] 校验：`cargo test --lib` → 503 passed

## Final Verification
- [x] `rg "Aligns with pi" src/` → zero matches
- [x] `cargo test --lib` → 503 passed
- [x] `cargo test --test bdd -- --test-threads=1` → 79 passed
