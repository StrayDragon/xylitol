# c279-relocate-executor-and-config-resolution Design

> 将 agent/ 中剩余的执行器、密钥解析、资源加载器和信任存储的具体 infra 依赖上提为 core ports，由组合根注入。目标：白名单 19 → 12，为 c278 聚焦 session 子系统清理舞台。

## 背景

c277 完成后，arch_guard 白名单剩余 19 项。其中 7 项落在 c279：

- `agent/runtime/bash.rs` 借用 4 个 `infra::` 原语（accumulator、process::kill_tree、truncate、process::shell::find_bash）。
- `agent/model/registry.rs` 直接调用 `infra::config::value::{resolve_config_value, resolve_headers}`。
- `agent/prompt/system.rs` 的 `build_system_prompt_from_loader` 直接引用 `infra::resource::DefaultResourceLoader`。
- `agent/session/mod.rs` 的 `save_trust_decision` 直接引用 `infra::trust::TrustManager`（c277 守卫修 bug 后新暴露）。

这些都不属于 agent 编排职责；把它们迁到 infra/ 并以 core port 注入，符合 HC-1/HC-2。

## 设计原则

1. **零行为变更**：只移动代码、抽取 trait、改注入路径；BDD 与现有测试不应感知差异。
2. **port 最小化**：每个 trait 只暴露 agent 必需的 API，不携带 infra 实现细节。
3. **组合根负责装配**：CLI / RPC / server 三个组合根统一构造 infra 实现并注入。
4. **不破坏既有测试**：`ModelRegistry::new()` 调用点多，统一改为 `ModelRegistry::new(resolver)`，同步更新测试 helper。

## 方案

### P1 — Bash executor 迁移（消 4 项白名单）

当前 `agent/runtime/bash.rs` 是一个完整的 async executor：进程派生、stdout/stderr 合并、取消、截断、temp spill。它同时被 `agent/session/bash_exec.rs` 调用。

#### 步骤

1. 在 `core::ports` 新增 `BashExecutor` port 与 `BashResult` 值类型：

   ```rust
   #[derive(Debug, Clone, Default)]
   pub struct BashResult {
       pub output: String,
       pub exit_code: Option<i32>,
       pub cancelled: bool,
       pub truncated: bool,
       pub full_output_path: Option<String>,
   }

   #[async_trait]
   pub trait BashExecutor: Send + Sync {
       async fn execute(&self, command: &str, cancel: Option<CancellationToken>) -> BashResult;
   }
   ```

2. 将 `parse_bang_prefix` 提升为 `core` 中的纯函数（无状态、不依赖 infra）：

   ```rust
   // src/core/bash.rs
   pub fn parse_bang_prefix(input: &str) -> Option<(bool, &str)>;
   ```

3. 新建 `src/infra/bash_exec/mod.rs`，把原 `agent/runtime/bash.rs` 的实现整体迁移为 `InfraBashExecutor`（或自由函数 + struct）。保留原单元测试。

4. 删除 `src/agent/runtime/bash.rs` 与 `src/agent/session/bash_exec.rs`。

5. `AgentSession` 改为持有：

   ```rust
   bash_executor: Arc<dyn BashExecutor>,
   bash_cancel: Option<CancellationToken>,
   ```

   `AgentSession::new` 新增参数 `bash_executor: Arc<dyn BashExecutor>`。

6. `AgentSession::execute_bash` 直接调用 `self.bash_executor.execute(command, cancel).await`；`abort_bash` 取消 token；`process_prompt` 调用 `core::bash::parse_bang_prefix`。

7. 组合根（cli/rpc/server）统一构造 `Arc::new(crate::infra::bash_exec::InfraBashExecutor::new())` 并传入 `Agent::with_ports` / `AgentSession::new`。

### P2 — SecretResolver port（消 1 项白名单）

当前 `agent/model/registry.rs` 的 `has_resolved_auth` 和 `resolve_provider_headers` 直接调用 `infra::config::value`。

#### 步骤

1. 在 `core::ports` 新增：

   ```rust
   pub trait SecretResolver: Send + Sync {
       fn resolve_config_value(&self, config: &str, env: Option<&HashMap<String, String>>) -> Option<String>;
       fn resolve_headers(
           &self,
           headers: &HashMap<String, String>,
           env: Option<&HashMap<String, String>>,
       ) -> Option<HashMap<String, String>>;
   }
   ```

2. 在 `infra/config` 提供 `InfraSecretResolver`（零状态包装）：

   ```rust
   pub struct InfraSecretResolver;
   impl SecretResolver for InfraSecretResolver { ... }
   ```

3. `ModelRegistry` 新增字段：`secret_resolver: Arc<dyn SecretResolver>`。

4. `ModelRegistry::new()` 改为 `ModelRegistry::new(resolver: Arc<dyn SecretResolver>)`。

5. 更新所有调用点（约 8 处，含 facade、rpc tests、session tests、registry tests 等）传入 `Arc::new(InfraSecretResolver)` 或共享实例。

6. `has_resolved_auth` / `resolve_provider_headers` 改为调用 `self.secret_resolver`。

### P3 — ResourceLoader port（消 1 项白名单）

`agent/prompt/system.rs::build_system_prompt_from_loader` 直接 import `infra::resource::DefaultResourceLoader`。

#### 步骤

1. 把纯数据类型 `AgentsFile` 从 `infra::resource::loader` 移到 `core::resource_types`。

2. 在 `core::ports` 新增 `ResourceLoader` trait，只暴露 prompt 构建所需方法：

   ```rust
   pub trait ResourceLoader: Send + Sync {
       fn get_agents_files(&self) -> &[AgentsFile];
       fn get_skills(&self) -> (&[SkillInfo], &[ResourceDiagnostic]);
       fn get_system_prompt(&self) -> Option<&str>;
       fn get_append_system_prompt(&self) -> &[String];
   }
   ```

3. `DefaultResourceLoader` impl `ResourceLoader`。

4. `agent/prompt/system.rs` 的 `build_system_prompt_from_loader` 参数改为 `&dyn ResourceLoader`，并删除 `crate::infra::resource::DefaultResourceLoader` import。

5. 测试里继续使用 `DefaultResourceLoader`，但通过 `&loader` 自动转换。

### P4 — TrustStore port（消 1 项白名单）

`AgentSession::save_trust_decision` 直接引用 `infra::trust::TrustManager`。

#### 步骤

1. 在 `core::ports` 新增：

   ```rust
   pub trait TrustStore: Send + Sync {
       fn set_trust(&self, path: &str, trusted: Option<bool>) -> Result<(), String>;
   }
   ```

2. `infra::trust::TrustManager` 实现 `TrustStore`。

3. `AgentSession::save_trust_decision` 签名改为：

   ```rust
   pub fn save_trust_decision(&self, trust_store: &dyn TrustStore, trusted: bool) -> Result<bool, String>
   ```

   内部调用 `trust_store.set_trust(&self.cwd, Some(trusted))?`。

### P5 — 收紧 arch_guard 白名单

完成 P1–P4 后，从 `src/tests.rs::AGENT_INFRA_ALLOWLIST` 删除 c279 的 7 项条目：

- `prompt/system.rs → crate::infra::resource`
- `model/registry.rs → crate::infra::config::value`
- `runtime/bash.rs → crate::infra::process::shell::find_bash`
- `runtime/bash.rs → crate::infra::tools::accumulator`
- `runtime/bash.rs → crate::infra::tools::process::kill_tree`
- `runtime/bash.rs → crate::infra::tools::truncate`
- `session/mod.rs → crate::infra::trust`

## 文件改动清单

| 文件 | 动作 |
|---|---|
| `src/core/ports.rs` | 新增 `BashResult`、`BashExecutor`、`SecretResolver`、`ResourceLoader`、`TrustStore` |
| `src/core/bash.rs` | 新建：纯函数 `parse_bang_prefix` |
| `src/core/resource_types.rs` | 移入 `AgentsFile` |
| `src/infra/bash_exec/mod.rs` | 新建：原 bash executor 实现 |
| `src/infra/config/secret_resolver.rs` | 新建：`InfraSecretResolver`（或合并到 `value.rs`） |
| `src/infra/resource/loader.rs` | impl `ResourceLoader` |
| `src/infra/trust/mod.rs` / `store.rs` | impl `TrustStore` |
| `src/agent/runtime/bash.rs` | 删除 |
| `src/agent/session/bash_exec.rs` | 删除 |
| `src/agent/session/mod.rs` | 注入 `bash_executor`、`TrustStore`；删除 trust infra import |
| `src/agent/model/registry.rs` | 注入 `SecretResolver` |
| `src/agent/prompt/system.rs` | 改用 `&dyn ResourceLoader` |
| `src/agent/facade.rs` | `Agent::with_ports` 新增 `bash_executor` 参数 |
| `src/interactive/cli/mod.rs` | 构造 `InfraBashExecutor` 并注入；构造 `InfraSecretResolver` 传给 `ModelRegistry` |
| `src/interactive/rpc.rs` | 同上 |
| `src/server/runtime.rs` | 同上 |
| `src/tests.rs` | 移除 c279 白名单条目 |

## 依赖与风险

- **依赖**：仅 `c277-sink-assembly-to-composition-root`（已完成）。
- **风险点**：
  - `ModelRegistry::new` 调用点多，容易漏改。编译器会报错，逐个修复即可。
  - `BashExecutor` port 的取消语义需要保持与现有 `BashExecHandler` 一致（取消后 kill_tree + 返回 `cancelled=true`）。
  - 三个组合根（cli/rpc/server）都需要注入新端口，需检查 server 是否用到 bash executor（目前 server 不处理 `!cmd`，但仍需注入 noop 或默认实现）。
- **回退策略**：若某 port 抽取导致 API 过度复杂，可在 design review 时将该单项 retag 到 c278；但当前方案保持最小改动，预计一次性完成。

## 验证

- `cargo build` 0 errors
- `cargo test` / `cargo nextest run --profile ci` 全绿
- `cargo test --test bdd -- --test-threads=1` 全绿
- `cargo clippy` 0 errors
- `cargo test --lib arch_guard` 全绿（白名单只剩 c278 12 项）
- `llman sdd validate c279-relocate-executor-and-config-resolution --strict --no-interactive` 通过
