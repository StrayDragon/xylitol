# Handoff: Unified Config Done → YAML Wiring Next

> 最后更新：2026-06-13 · clippy 0 warnings · 245 + 77 tests pass

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
| src 源文件 | 73 个 |
| 总代码行数 | ~17,500L (含 ~3,100L 死代码) |
| clippy warnings | 0 |

## 本次会话 Commit 历史

| Hash | 说明 |
|------|------|
| `a4d1489` | chore: remove lspz dependency and infra-lsp/dap modules (-1110L) |
| `7b22fd0` | refactor(agent): merge duplicate ModelRegistry into single canonical version |
| `6c7ddc8` | fix(agent): ReAct loop sends full history every turn; wire CancellationToken |
| `67a6bb9` | refactor(provider): rewrite OpenAI provider with async-openai crate |
| `2d493f6` | docs: update handoff with 5 critical fixes summary |
| `ee4f70b` | docs: update handoff and next with dead code triage plan |
| `*` | **refactor(model): unify ModelKind/ModelConfig, eliminate ProviderKind** |

## 2026-06-13 最新: 配置系统统一

### ✅ 已完成: ModelKind / ModelConfig 统一

```
Before:                                    After:
────────────────────────────────────────── ──────────────────────────────────
agent::model::ModelKind     (3 variants)   → 统一在这里（加 serde/schemars）
infra::config::types::ProviderKind (2 var)  → 删除，合并到 ModelKind

agent::model::ModelConfig    (运行时)       → 唯一的运行时 ModelConfig
infra::config::types::ModelConfig (YAML)    → 重命名为 ModelsConfig

ModelEntry.provider: ProviderKind           → ModelEntry.provider: ModelKind
```

### 🔴 待处理: CLI 未调用 YAML 配置系统

```
当前流程:                      应该的流程:
  env vars                       config.yaml (5-layer merge)
    ↓                                ↓
  CLI 硬编码 ModelRegistry()    load_app_config() → AppConfig
    ↓                                ↓
  AgentSession::new()           resolve_model() → agent::model::ModelConfig
                                    ↓
                               AgentSession::new()
```

YAML 配置系统 (loader.rs, types.rs, paths.rs, secret.rs, template.rs, validate.rs) 代码完整但 `interface/cli/mod.rs` 从未调用 `load_app_config()` — CLI 目前完全跳过配置层直接从 env vars 构建。

### 死代码 (保留给 TUI)

按你的要求，pi-parity 死代码先保留（TUI 阶段会用到）：
- `trust.rs` / `project_trust.rs` — 信任决策（TUI 交互需要）
- `commands.rs` / `resolver.rs` / `templates.rs` — 交互式命令
- `output_guard.rs` / `event.rs` — 界面输出控制
- `resource.rs` — 项目上下文加载

### 误标记 dead_code 已修复 (B 阶段 ✅)

- `agent/retry.rs` — 去掉 `#![allow(dead_code)]`，行级标注未使用项
- `agent/templates.rs` — 同上
- `infra/config/secret.rs` / `template.rs` / `validate.rs` — 保留（config 系统待接线）

## 下一步

| # | 任务 | 优先级 |
|---|------|--------|
| 1 | CLI 接入 YAML 配置 (`load_app_config()` → `resolve_model()`) | 🔴 最高 |
| 2 | `AppConfig::model` 字段名改为 `models`（更准确） | 🟡 建议 |
| 3 | `pub` → `pub(crate)` 收紧 | 🟡 |
| 4 | 撰写 `docs/architecture.md` | 🟡 |
