# Handoff: Code Audit Complete ✅ + YAML Config Wired ✅

> 最后更新：2026-06-15 · clippy 0 warnings · 245 + 77 tests pass
> 审计进度: agent/ ✅, infra/ ✅, interface/ ✅ · deps ✅ · visibility ✅ · unsafe ✅ · arch doc ✅

## 当前测试状态

```bash
cargo test --test bdd -- --test-threads=1  # → 77 passed ✅
cargo test --lib                             # → 245 passed + 1 ignored ✅
cargo fmt -- --check                         # → clean ✅
cargo clippy --all-targets                   # → 0 warnings ✅
```

| 指标 | 数值 |
|------|------|
| lib tests | 245 + 1 ignored |
| BDD scenarios | 77 |
| total (default features) | 323 |
| clippy warnings | 0 |

> 默认 features: `infra-skills`, `infra-session`, `ui-review`。
> `infra-lsp` 不在默认编译中（28 个 lsp 测试需显式启用 feature）。

## 阶段状态: 审计完成 ✅

代码审计阶段已全部完成 (2026-06-15)。

### 已完成

1. ✅ 修复 27 个 clippy warnings
2. ✅ 代码审计 (`src/agent/`, `src/infra/`, `src/interface/`)
3. ✅ 架构优化 (降低耦合、精简抽象、移除死代码)
4. ✅ 加固测试质量
5. ✅ 撰写 `docs/architecture.md` (330 行, 17 章)

### 审计细节

1. [x] 修复 27 clippy warnings
2. [x] 对齐 Cargo.toml 默认 features → `infra-skills`, `infra-session`, `ui-review`
3. [x] 审计 agent/ 层 — `.unwrap()` → `.expect()` (8) + 移除 `#![allow(dead_code)]` + 标注 2 个死字段
4. [x] 审计 infra/ 层 — `.unwrap()` → `.expect()` (5), 移除 4 `#[allow]`, 删除 296L 死代码
5. [x] 审计 interface/ 层 — 删除 acp.rs (死代码, 3L), 移除 diff_review/types.rs `#![allow(dead_code)]`, cli/mod.rs 消除硬编码 model name
6. [x] 识别并移除死代码 — 296L 已删 + acp.rs
7. [x] 审查 `pub` vs `pub(crate)` 可见性 — 556 pub : 365 pub(crate) (was 335:1)
8. [x] 审查错误处理 — 所有非测试 `.unwrap()` → `.expect()`
9. [x] 审查依赖树 — 移除 4 未使用 crate, 替换 deprecated serde_yaml → yaml_serde, 32→31 direct deps
10. [x] 审查 `unsafe` — 13 全在 tests, 已加 SAFETY 注释 + test_support.rs 模块级文档
11. [x] 撰写 `docs/architecture.md` — 330 行, 17 章, 完整架构 SSOT

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

## 近期 Commit 历史

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

## 近期完成摘要

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

## 下一步

| # | 任务 | 优先级 |
|---|------|--------|
| 1 | `AppConfig::model` 字段重命名为 `models` | 🟡 建议 |
| 2 | 进入下一开发阶段 (TBD) | — |
