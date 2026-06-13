# Handoff: Code Audit & Deep Fixes Done → Dead Code Triage Next

> 最后更新：2026-06-13 · clippy 0 warnings · Core fixes complete

## 当前测试状态

```bash
cargo test --test bdd -- --test-threads=1  # → 77 passed ✅
cargo test --lib                             # → 245 passed + 1 ignored ✅
cargo fmt -- --check                         # → clean ✅
cargo clippy --all-targets                   # → 0 warnings ✅
```

| 指标 | 数值 |
|------|------|
| lib tests | 245 |
| BDD scenarios | 77 |
| total | 322 (BDD: 77, lib: 245) |
| src 源文件 | 73 个 (lsp/dap 已删除, openai.rs 重写为 async-openai) |
| 总代码行数 | ~17,500L |
| 实际活跃代码 | ~14,400L |
| 死代码 (标记 allow(dead_code)) | ~3,100L 分布在 13 个模块 |
| clippy warnings | 0 |

## Commit 历史（本次会话）

| Hash | 说明 |
|------|------|
| `a4d1489` | chore: remove lspz dependency and infra-lsp/dap modules (-1110L) |
| `7b22fd0` | refactor(agent): merge duplicate ModelRegistry into single canonical version |
| `6c7ddc8` | fix(agent): ReAct loop sends full history every turn; wire CancellationToken |
| `67a6bb9` | refactor(provider): rewrite OpenAI provider with async-openai crate |
| `2d493f6` | docs: update handoff with 5 critical fixes summary |

## 死代码现状分析

### ❌ 确认未使用可删除 (13 个模块, ~3,056L, 78 测试)

| 模块 | 行数 | 测试 | 说明 |
|------|------|------|------|
| `agent/trust.rs` | 528 | 11 | TrustManager — 从未被 AgentSession 或 loop 调用 |
| `agent/project_trust.rs` | 393 | 11 | ProjectTrust store — 完全孤立 |
| `agent/resolver.rs` | 496 | 14 | ModelResolver — CLI 不使用,用简单 match 代替 |
| `agent/commands.rs` | 172 | 9 | SlashCommands — process_prompt()存在但CLI从未调用 |
| `agent/diagnostics.rs` | 205 | 6 | 诊断收集器 — 未连线 |
| `agent/output_guard.rs` | 170 | 8 | OutputGuard — session 有方法但 print.rs 不调用 |
| `agent/event.rs` | 110 | 3 | EventBus — AgentLoop 不通过它发布事件 |
| `agent/queue.rs` | 77 | 0 | MessageQueue — session 有字段但从不使用 |
| `agent/defaults.rs` | 62 | 5 | 默认值定义 — 未使用 |
| `infra/resource.rs` | 358 | 9 | ResourceLoader — AgentSession 从不加载AGENTS.md |
| `infra/session/fine_tune.rs` | 244 | 7 | Fine-tune — 未连线 |
| `infra/session/storage.rs` | 174 | 4 | Storage — 未被 manager 使用 |
| `infra/session/gc.rs` | 13 | 0 | GC — 空壳 |
| `infra/session/config.rs` | 67 | 0 | SessionConfig — 未连线 |

### ⚠️ 已使用但标记为 dead_code (5 个)

| 模块 | 行数 | 说明 |
|------|------|------|
| `agent/retry.rs` | 84 | loop.rs 实际使用 `RetryState` + `is_retryable_error` |
| `agent/templates.rs` | 318 | session.rs 导入 `is_template_line`/`parse_template_line` |
| `infra/config/secret.rs` | 135 | loader.rs 使用 `load_secret_env` |
| `infra/config/template.rs` | 143 | loader.rs 使用 `render` |
| `infra/config/validate.rs` | 139 | loader.rs 使用 `validate_config` |
| `infra/config/loader.rs` | 405 | 仅被 CLI 间接使用,内部标记了 dead_code |

> 这些模块的 `#![allow(dead_code)]` 应移除（或降级为行级 `#[allow(dead_code)]`）。

## 下一步计划

### 阶段 A: 死代码清理 (删除 3,056L)
删除上表 13 个确认未使用的模块。每个模块独立删除，测试仍然全部通过。

### 阶段 B: 收紧 `#![allow(dead_code)]`
移除或降级 5 个实际使用模块的 `#![allow(dead_code)]`。

### 阶段 C: 可见性收紧
当前 133 个 `pub` (对外暴露) vs 365 个 `pub(crate)`。需要审查哪些字段/struct 不应对外暴露。

### 阶段 D: 撰写 architecture.md
以清理后的代码为基准，文档化最终架构。
