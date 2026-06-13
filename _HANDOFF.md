# Handoff: YAML Config Wired ✅

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
| clippy warnings | 0 |

## 本次会话所有 Commit

| Hash | 说明 |
|------|------|
| `a4d1489` | chore: remove lspz dependency and infra-lsp/dap modules |
| `7b22fd0` | refactor(agent): merge duplicate ModelRegistry |
| `6c7ddc8` | fix(agent): ReAct loop + CancellationToken |
| `67a6bb9` | refactor(provider): rewrite OpenAI with async-openai |
| `2d493f6` | docs: handoff update |
| `ee4f70b` | docs: dead code triage plan |
| `691c7b3` | refactor(model): unify ModelKind/ModelConfig, B phase |
| `1a2288c` | docs: handoff + next update |
| `*` | **feat(cli): wire YAML config loader into CLI** |

## 本次会话完成摘要

| 类别 | 内容 |
|------|------|
| 🔴 Bug 修复 | ReAct loop 多轮丢上下文、CancellationToken 悬空 |
| 🔴 架构 | 双重 ModelRegistry 合并、ModelKind 统一、ProviderKind 删除 |
| 🟡 依赖 | lspz + infra-lsp/dap 删除、async-openai 接入 |
| 🟢 配置 | YAML 5层加载 → CLI → ModelRegistry 全链路打通 |
| 🟢 注解 | retry.rs + templates.rs #![allow(dead_code)] 修复 |

## 配置: YAML → CLI 完整数据流

```
config.yaml                config.local.yaml        --config override
    ↓                            ↓                       ↓
load_app_config() → deep merge → MiniJinja render → JSON schema validate
    ↓
ModelsConfig { models: { alias → ModelEntry { provider, model, ... } } }
    ↓
for each (alias, entry): resolve_api_key() → ModelMeta → ModelRegistry
    ↓                                                    (fallback: env vars)
AgentSession::new(registry, ...)
```

用户只需创建 `~/.config/xylitol/config.yaml`:

```yaml
model:
  default_model: gpt-4o
  models:
    gpt-4o:
      provider: openai
      model: gpt-4o
    sonnet:
      provider: anthropic
      model: claude-sonnet-4-20250514
      context_window: 200000
```

`xylitol --model sonnet "hello"` 即可工作。API key 仍从环境变量读取 (`OPENAI_API_KEY` / `ANTHROPIC_API_KEY`)。

## 死代码: 保留给 TUI

按决策保留 pi-parity 模块 (TUI 阶段接入):
- `trust.rs` / `project_trust.rs` — 信任决策
- `commands.rs` / `resolver.rs` / `templates.rs` — 交互命令
- `output_guard.rs` / `event.rs` — 界面输出
- `resource.rs` — 项目上下文

## 下一步

| # | 任务 | 优先级 |
|---|------|--------|
| 1 | `AppConfig::model` 字段重命名为 `models` | 🟡 建议 |
| 2 | `pub` → `pub(crate)` 收紧 | 🟡 |
| 3 | 撰写 `docs/architecture.md` | 🟡 |
