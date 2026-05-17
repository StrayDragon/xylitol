<!-- LLMANSPEC:START -->
# LLMAN 规范驱动开发

本项目使用 llman SDD。阅读 `llmanspec/config.yaml` 了解项目上下文与规则。

使用 `/llman-sdd-onboard` 开始，然后使用 `/llman-sdd-*` 技能进行工作流。

保留此托管块，便于 `llman sdd update` 刷新。
<!-- LLMANSPEC:END -->

# AGENTS.md

> **SSOT**: 本文档是 xylitol 项目的通用规范文档。

---

## 项目简介

xylitol 是一个 Rust 编写的 AI coding agent，基于 [adk-rust](https://github.com/StrayDragon/adk-rust) 框架构建，参考 codex-rs 设计。单 crate + 领域分层 + feature flags 架构。

---

## 架构

```
src/
├── agent/       # 核心领域：agent loop, tools, config, prompts (adk-core + adk-agent + adk-runner)
├── infra/       # 基础设施：hooks, security, repeat, lsp, skills, session, planning, dap
└── interface/   # 用户接口：cli, print, tui, rpc, review
```

层间通过 `pub(crate)` 控制可见性，跨层访问通过 `lib.rs` re-export。

### adk-rust 依赖映射

| xylitol 层 | adk-rust crate | 用途 |
|-------------|----------------|------|
| `agent/` | `adk-core` + `adk-agent` + `adk-runner` | Agent trait, LlmAgent ReAct 循环, Runner 生命周期 |
| `agent/model` | `adk-model` | LLM Provider（MVP 仅 OpenAI + Anthropic） |
| `infra/session` | `adk-session` | SQLite 后端, event compaction |
| `infra/skills` | `adk-skill` + `adk-tool` | Skill 发现+注入, MCP 客户端 (rmcp) |
| `infra/sandbox` | `adk-sandbox` | OS 级沙箱 (Seatbelt/seccomp) |
| `infra/eval` | `adk-eval` | MockLlm, 轨迹评分, LLM-as-judge |
| `interface/cli` | `adk-cli` | Launcher + StreamPrinter（扩展） |

---

## Feature Flags

命名规则：`<domain>-<capability>`

### 三层策略

- **核心**（始终编译）：Agent 循环, 7 内置工具, LLM Provider, Config, CLI + Print 模式
- **内置增强**（始终编译，运行时 config 开关）：Hook 系统, 安全策略, 重复检测, Patch Apply
- **可选扩展**（Cargo feature flag）：

| Feature | 层 | 重依赖 |
|---------|-----|--------|
| `agent-planning` | agent | — |
| `infra-lsp` | infra | `lspz` |
| `infra-skills` | infra | `adk-tool`（rmcp） |
| `infra-session` | infra | `adk-session`（rusqlite） |
| `infra-dap` | infra | Phase 2 |
| `ui-tui` | interface | `ratatui`, `crossterm`, `termimad`, `syntect` |
| `ui-review` | interface | `syntect`, `similar`, `axum` |
| `ui-rpc` | interface | — |
| `dev-vt100` | dev | `vt100` |
| `dev-e2e` | dev | — |

**默认 features**: `ui-tui`, `infra-session`, `ui-review`

---

## Rust

- Edition 2024, stable toolchain (`rust-toolchain.toml`)
- Nightly-only options are not allowed
- `#![cfg_attr(docsrs, warn(missing_docs))]` — 公共 API 必须文档化
- `thiserror` 定义错误枚举，`anyhow` 应用层。避免 `.unwrap()` / `.expect()`（测试除外）
- `tokio` 运行时，`async-trait` 异步 trait，所有 I/O 异步
- `tracing`（非 `log`），结构化日志

---

## 工具链

```bash
just setup    # 安装 prek hooks
just qa       # fmt-check + lint + test + doc-check
just ci       # prek run --all-files + qa
```

- **Git Hooks** (`prek.toml`): pre-commit(cargo-fmt + clippy + builtin), commit-msg(conventional commits), pre-push(cargo-test). Zero network dependency.
- **Commits**: `type(scope)!: description`，max 72 chars。Types: feat/fix/docs/style/refactor/perf/test/build/ci/chore/revert
- **测试**: 单元(源码 `tests` 模块), 集成(`tests/`), 性能(`benches/`). 命名 `test_<功能>_<场景>`. 异步必须 `tokio::time::timeout`
- **构建**: dev=`opt-level 0`+依赖`3`, release=`opt-level "z"`+`lto thin`+`strip symbols`+`panic abort`
- **Shell**: `bash -euo pipefail`

---

## 版本管理

`v0.0.0-dev` crate name locking. 阶段完成：`just qa` → 提交 → 版本号 → tag. tag 后禁止修改历史。

---

## llman SDD 工作流

- 依赖关系在 `llmanspec/changes/<id>/proposal.md` 的 YAML frontmatter（`depends_on` / `blocks`）中声明
- 处理前检查 `depends_on`，DAG 可视化：`llman sdd graph`

---

## MVP Change DAG

```
c05-init-skeleton ──────────────────────────────────────────────
  ├─ c10-add-config ────────────────────────────────────────────
  │   ├─ c15-add-cli ──────────────────────────────────────────
  │   │   ├─ c20-add-tools ────────────────────────────────────
  │   │   │   └─ c25-add-agent-loop ───────────────────────────
  │   │   │     ├─ c30-add-print-mode
  │   │   │     ├─ c35-add-repeat-detection
  │   │   │     ├─ c55-add-planning-execution (agent-planning)
  │   │   │     │   └─ c60-add-model-lock (Phase 2)
  │   │   │     ├─ c70-add-session-snapshot (infra-session)
  │   │   │     └─ c88-add-test-infra
  │   │   ├─ c80-add-tui (ui-tui)
  │   │   └─ c87-add-rpc-mode
  │   ├─ c40-add-hooks ── c50-add-security
  │   ├─ c45-add-lsp-layer (infra-lsp)
  │   ├─ c65-add-skills-mcp (infra-skills)
  │   ├─ c75-add-diff-review (ui-review)
  │   └─ c85-add-dap-layer (Phase 2)
```

---

## 参考

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Conventional Commits](https://www.conventionalcommits.org/)
- [adk-rust](https://github.com/StrayDragon/adk-rust)
