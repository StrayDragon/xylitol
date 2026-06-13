# Handoff: Code Audit & Deep Fixes Done

> 最后更新：2026-06-13 · clippy 0 warnings · 5 个关键问题已修复

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
| total | 322 |
| src 源文件 | ~63 个 (lsp/dap 已删除 ~1100L, openai.rs 从 398L 减为 350L) |
| clippy warnings | 0 |

> 默认 features 已对齐为 `infra-skills`, `infra-session`, `ui-review`。
> `infra-lsp`, `infra-dap` 已完全移除（含源码和 feature flag）。
> `lspz` 依赖已删除。

## 阶段状态: 功能开发已冻结

**不再实现新功能。** 当前阶段聚焦:

1. 修复 27 个 clippy warnings
2. 代码审计 (`src/agent/`, `src/infra/`, `src/interface/`)
3. 架构优化 (降低耦合、精简抽象、移除死代码)
4. 加固测试质量
5. 撰写 `docs/architecture.md`

### 已明确不做 (permanent out-of-scope)

| Category | 说明 |
|----------|------|
| PackageManager 检测 | 用户自行管理依赖 |
| OAuth / auth-storage | 用户自行配置 API key |
| Extensions SDK | 不实现 |
| 额外 provider (Gemini, Ollama) | 仅 OpenAI-like + Anthropic-like |
| 多模态输入 | 不规划 |
| TUI / GUI / Web | 交互形态待定 |

### 交互形态: TBD

当前仅 CLI 单次模式 (`print`)。未来方向待决策。

## 2026-06-13: 5 个关键问题修复

### ✅ P0: 双重 ModelRegistry 已合并
- 删除 `session.rs` 中的简单版 `ModelRegistry`，改从 `registry.rs` 导入
- 复杂版 `ModelRegistry` 从 `pub(crate)` 升级为 `pub`，新增 `get_at()` / `index_of()` 方法
- `AgentSession` 不再直接访问 `.models` 内部字段

### ✅ P0: ReAct Loop 多轮逻辑错误已修复
- `run_react_loop()` 每个 turn 现在发送完整 `history`（含之前 assistant + tool results）
- 添加了 `ReActConfig` struct 解决 clippy "too many arguments"

### ✅ P0: OpenAI provider 已用 async-openai 重写
- 从 ~398 行手写 HTTP+SSE 降为 ~350 行，利用 `async_openai::Client` 的 Chat + Stream API
- Cargo.toml 启用 `chat-completion + rustls` features

### ✅ P0: LSP/DAP 已完全移除
- 删除 `src/infra/lsp/` + `src/infra/dap/` (~1100L)
- 从 `Cargo.toml` 删除 `lspz` 依赖和 `infra-lsp`/`infra-dap` features

### ✅ P0: CancellationToken 已正确连线
- `AgentLoop` 持有 `CancellationToken`，通过 `abort()` 公开取消接口
- tool 调用及每 turn 开头都检查 `cancel.is_cancelled()`
- 工具执行使用 `XyToolCtx::with_cancel()` 传递父 token

## 变更历史

| # | 变更 | 状态 | 说明 |
|---|------|------|------|
| c05 | rebuild-core | ✅ 归档 | 7 tools, ReAct loop, session, hooks, CLI |
| c06 | update-bdd-framework | ✅ 归档 | cucumber-rs → rstest-bdd |
| c07 | fix-bdd-scenarios | ✅ 归档 | 36 个 BDD 失败修复, 77/77 全绿 |
| c25 | phase3-infra-gaps | ✅ 归档 | ModelRegistry/Resolver, ResourceLoader, PromptTemplate, SlashCommands, OutputAccumulator, defaults, diagnostics, SessionCWD |
| — | llm-compaction | ✅ 代码完成 | compaction.rs (1217L) |
| — | streaming-cancel | ✅ 代码完成 | CancellationToken + tokio::select! |
| — | session-fork | ✅ 代码完成 | SessionManager::fork() |
| c26 | add-outputguard-lifecycle | ✅ 归档 | OutputGuard, AgentSession lifecycle |
| — | feat(trust) | ✅ 提交 | trust.rs (529L) + project_trust.rs (394L) |

## 活跃变更: 无

## 文件结构概览

```
src/
  agent/          — loop, session, trust, project_trust, registry, resolver,
                    tools (7 built-in), templates, commands, output_guard,
                    defaults, diagnostics, system prompt, event, queue,
                    provider (OpenAI + Anthropic + mock + fake)
  infra/          — hooks (pre/post), skills (MCP), resource (AGENTS.md walk-up),
                    session (compaction, storage, manager, gc, fine_tune),
                    config (loader, paths, secret, template, validate)
  interface/      — cli, print, diff_review
tests/
  bdd.rs          — BDD step definitions, 77 scenarios
  features/       — 12 .feature files
  support/        — test harness
llmanspec/
  specs/          — 24 specs
  changes/archive/— archived changes
  changes/not-planning/ — deferred: c60-add-model-lock, c85-add-dap-layer
```

## 下一步行动

1. [x] 修复 27 clippy warnings
2. [x] 对齐 Cargo.toml 默认 features
3. [x] 审计 agent/ 层
4. [x] 审计 infra/ 层
5. [x] 审计 interface/ 层
6. [x] 识别并移除死代码
7. [x] 合并双重 ModelRegistry
8. [x] 修复 ReAct Loop 多轮 bug
9. [x] 用 async-openai 重写 OpenAI provider
10. [x] 移除 LSP/DAP
11. [x] 修复 CancellationToken 连线
12. [ ] 审查 `pub` vs `pub(crate)` 可见性
13. [ ] 审查 `unsafe` (13 个全在 tests, 需加注释)
14. [ ] 撰写 `docs/architecture.md`
15. [ ] 决定 pi-parity 死代码模块去留: `trust.rs` (529L) + `project_trust.rs` (394L) + `output_guard.rs` (77L) + `commands.rs` (173L) + `resolver.rs` (496L) + `templates.rs` (318L) + `resource.rs` (358L) + `diagnostics.rs` (206L) + `event.rs` (110L) + `queue.rs` — 合计 ~3000L
