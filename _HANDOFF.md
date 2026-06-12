# Handoff: Code Audit In Progress ▸ Phase 1 Interface Done

> 最后更新：2026-06-12 · 最新 commit: 即将 · clippy 0 warnings
> 审计进度: agent/ ✅, infra/ ✅, interface/ ✅ · deps ✅

## 当前测试状态

```bash
cargo test --test bdd -- --test-threads=1  # → 77 passed ✅
cargo test --lib                             # → 251 passed + 1 ignored ✅
cargo fmt -- --check                         # → clean ✅
cargo clippy --all-targets                   # → 0 warnings ✅ (27 fixed)
```

| 指标 | 数值 |
|------|------|
| lib tests | 251 |
| BDD scenarios | 77 |
| total | 328 |
| src 源文件 | ~70 个，~19,000 行 |
| clippy warnings | 0 (27 resolved) |

> 默认 features 已对齐为 `infra-skills`, `infra-session`, `ui-review`。
> `infra-lsp` 已移出默认编译（28 个 lsp 测试不再编译，251 + 77 = 328）。

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
  interface/      — cli, print, acp, diff_review
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
2. [x] 对齐 Cargo.toml 默认 features → `infra-skills`, `infra-session`, `ui-review`
3. [x] 审计 agent/ 层 — `.unwrap()` → `.expect()` (8) + 移除 `#![allow(dead_code)]` + 标注 2 个死字段
4. [x] 审计 infra/ 层 — `.unwrap()` → `.expect()` (5), 移除 4 `#[allow]`, 删除 296L 死代码
5. [x] 审计 interface/ 层 — 删除 acp.rs (死代码, 3L), 移除 diff_review/types.rs `#![allow(dead_code)]`, cli/mod.rs 消除硬编码 model name (用 registry::default_model_id_for_provider)
6. [x] 识别并移除死代码 — 296L 已删 + acp.rs
7. [ ] 审查 `pub` vs `pub(crate)` 可见性 (335 pub, 1 pub(crate))
8. [x] 审查错误处理 — 所有非测试 `.unwrap()` → `.expect()`
9. [x] 审查依赖树 — 移除 4 未使用 crate, 替换 deprecated serde_yaml → yaml_serde, 32→31 direct deps
10. [ ] 审查 `unsafe` (13 个全在 tests, 需加注释)
11. [ ] 撰写 `docs/architecture.md`
