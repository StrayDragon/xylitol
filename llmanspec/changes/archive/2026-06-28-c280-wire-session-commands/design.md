# c280-wire-session-commands Design

> 补齐三条断裂的接线：RPC 会话命令的真实实现、AGENTS.md/系统提示/skills 接入 live
> prompt、死 slash 命令层的清理。所有改动基于 c278 已就位的 `SessionStore` port。

## 背景

### 通道真相

项目有**两条**命令体系，但只有一条是活的：

| 体系 | 状态 | 证据 |
|---|---|---|
| **RPC `Command` 枚举**（protocol.rs）→ rpc.rs `dispatch` | 🟢 活（大半） | ExportHtml/Compact/Fork/GetSessionStats 等都有真实 dispatch |
| **slash `BUILTIN_COMMANDS`**（commands.rs）→ `process_prompt` → `PromptResult::Handled` | 🔴 死 | `process_prompt` 在 interactive/runtime/server **零调用方**；`Handled` 分支无消费 |

c278 删除零调用的 `resume_session`/`switch_session` 方法时暴露了这一点。本变更顺势收口：
不复活死系统，而是补齐活系统（RPC）的缺口，并删除死系统。

### 三条断裂接线

**接线 1 — `SwitchSession` 假实现**（rpc.rs）：
```rust
Command::SwitchSession { session_path, .. } => {
    s.session_id = session_path.clone();  // 仅改字符串，无 load/validate
}
```
c278 删的 `AgentSession::switch_session` 至少做了 `switch_session` + `set_active_session` +
CWD 校验。当前 RPC 比它更弱。

**接线 2 — `GetMessages` stub + Export/Import JSONL 缺失**：
- `GetMessages` 返回 `"not yet implemented in RPC"`
- `ExportJsonl`/`ImportJsonl` 不在 `Command` 枚举，但 `AgentSession` 有活的
  `export_to_jsonl`/`import_from_jsonl` 方法（c278 内联后的 core 调用）

**接线 3 — AGENTS.md/系统提示/skills 未接入**：
cli/mod.rs 构造 `DefaultResourceLoader` 但只取 `get_prompts()`。`AgentSession::new` 的
`prompt_opts` 仅设 `cwd`：
```rust
prompt_opts: SystemPromptOpts { cwd, ..Default::default() },  // context_files/system_prompt/skills 全空
```
导致 `build_system_prompt` 用硬编码 `"You are a helpful AI assistant."`，AGENTS.md 内容
被完全忽略。

## 设计原则

1. **补齐活系统，不复活死系统**：所有新能力走 RPC `Command` → `AgentSession` 方法。
   `process_prompt` 的 slash 死分支删除。
2. **零行为回归**：既有 88 BDD + lib 测试不退化；硬编码 fallback 移除前确认 loader
   默认提示等价。
3. **组合根负责装配**：CLI 在构造 `AgentSession` 时一次性填充 `prompt_opts`（读 loader），
   agent 不直接持有 loader（保持 c278 的零 infra 持有）。
4. **port 最小扩容**：若 `SwitchSession` 需要 CWD 校验，优先把 `load_validated` 上提为
   `SessionStore` port 方法（c278 已有 `load_entries`，校验是纯函数叠加）。

## 方案

### P1 — RPC 会话命令补齐（interactive-protocol）

#### P1.1 — `SwitchSession` 真实化

当前仅改字符串。目标：校验目标会话存在 → 切换 session_id → 触发上下文重建。

```rust
Command::SwitchSession { session_path, .. } => {
    let mut s = state.lock().await;
    let new_id = derive_session_id(&session_path);  // path → stem
    let store = s.store();  // 需要 RpcState 暴露 store
    if !store.exists(&new_id).await {
        emit Err "session not found: {new_id}"; return;
    }
    s.session_id = new_id.clone();
    // 触发 AgentSession 重建（ensure_agent 会按新 session_id 构建）
    s.invalidate_cached_agent();
    emit Response { session: new_id, ... };
}
```

**关键决策**：`RpcState` 当前缓存 `cached_agent`。switch 后必须失效缓存，否则
`ensure_agent` 复用旧 session 的 agent。需确认 `invalidate_cached_agent` 逻辑存在。

#### P1.2 — `GetMessages` 实现

```rust
Command::GetMessages { .. } => {
    let s = state.lock().await;
    let store = s.store();
    let entries = store.load_entries(&s.session_id).await?;
    emit Response { messages: entries };  // SessionEntry 序列化
}
```
`SessionEntry` 已 `Serialize`（c276 上提 core），protocol 直接序列化。

#### P1.3 — 新增 `ExportJsonl` / `ImportJsonl`

与 `ExportHtml`（rpc.rs:464）对称，`Command` 枚举加两变体，dispatch 转发到
`AgentSession::export_to_jsonl` / `import_from_jsonl`。`ImportJsonl` 返回新 session_id。

### P2 — AGENTS.md / 系统提示 / skills 接入（agent-session）

#### P2.1 — CLI 组合根填充 prompt_opts

cli/mod.rs 已构造 `loader`（line 338）。改为复用同一 loader 取全部字段：

```rust
let loader = DefaultResourceLoader::new(loader_cwd, agent_dir);
let discovered_templates = loader.get_prompts().0.to_vec();
// 新增：填充系统提示上下文
let context_files: Vec<(String, String)> = loader_field(&loader, AgentsFile)
    .iter().map(|f| (f.path.to_string_lossy().into(), f.content.clone())).collect();
let system_prompt_text = loader.get_system_prompt().map(String::from);
let append_system = loader.get_append_system_prompt().to_vec();
```

但注意：`get_agents_files`/`get_system_prompt`/`get_append_system_prompt` 在 c280 前的
卫生清理中从 inherent 方法删为**仅 trait impl**。需要经 `&dyn ResourceLoader` 或恢复
inherent 方法。**决策：恢复 inherent 方法**（interactive/resources.rs 已直接调 inherent
方法如 `get_skills`，trait 路径反而是多余的）。

#### P2.2 — AgentSession::new 接受填充后的 opts

当前 `new` 收 `system_prompt: Option<String>` 单独参数，内部组装 `prompt_opts`。改为
接受更完整的上下文，或新增 builder 方法。**最小改动**：`new` 增参 `context_files`/
`append_system_prompt`（与现有 `system_prompt` 参数并列）。

#### P2.3 — 移除硬编码 fallback

`cli/mod.rs:277` 的 `.unwrap_or_else(|| "You are a helpful AI assistant.".into())`
改为：loader 无 system_prompt 时传 `None`，由 `build_system_prompt` 的内置默认接管
（`build_system_prompt` 已有 `"You are an expert coding assistant..."` 兜底，见
system.rs:test_build_basic_prompt）。

### P3 — 删除死 slash 命令系统（agent-session）

确认后删除：
- `process_prompt` 的 `/command` → `Handled` 分支（保留 `!cmd`/`/template:`/`/skill:`）
- `PromptResult::Handled` 变体（若无其他消费方）
- `commands.rs` 的 `is_slash_command` / `get_command_args`（若仅 process_prompt 用）
- 保留 `BUILTIN_COMMANDS` 表 + `get_all_commands`（`GetMessages`/`GetCommands` RPC 的数据源）

## 风险矩阵

| 风险 | 影响 | 缓解 |
|---|---|---|
| SwitchSession 失效缓存逻辑不存在 | 切换后用旧 agent | 检查 RpcState.invalidate；必要时新增 |
| 恢复 inherent getter 与 trait impl 重复 | clippy 噪音 | inherent 调用为默认；trait impl 保留供未来 dyn |
| AgentSession::new 签名再变 | 调用点多 | 统一更新 cli/rpc/server/tests |
| 删除 slash 系统误伤活路径 | /template: /skill: 失效 | 仅删 `/command`→Handled，保留其余分支 |
| AGENTS.md 内容影响现有 BDD | prompt 变化致快照漂移 | BDD 用 tmp dir，无 AGENTS.md，不受影响 |

## 验证策略

1. `cargo nextest run -p xylitol --lib` 全绿。
2. `cargo test --test bdd -- --test-threads=1` 88→90+（新增 switch/export-jsonl BDD）。
3. `cargo clippy -- -D warnings`（just lint 新闸门）。
4. 手测：在含 AGENTS.md 的目录跑 CLI，确认系统提示含其内容。
5. protocol.rs `Command` 枚举 snapshot/测试更新。

## Out of Scope

- 本地交互式 REPL（read-eval loop）——当前 CLI 是一次性 print 模式，不在本变更范围。
- slash 命令的交互式 dispatch 复活（明确放弃，RPC 是真实通道）。
- 会话树管理 UI（`/tree` 命令的 navigate）——保留为后续。
