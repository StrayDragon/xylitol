<!-- LLMANSPEC:START -->
# LLMAN 规范驱动开发

本项目使用 llman SDD。阅读 `llmanspec/config.yaml` 了解项目上下文与规则。

使用 `/llman-sdd-onboard` 开始，然后使用 `/llman-sdd-*` 技能进行工作流。

保留此托管块，便于 `llman sdd update` 刷新。
<!-- LLMANSPEC:END -->

请先阅读 @AGENTS.md 获得基本认知, 然后根据以下实践阶段覆盖为准

> **SSOT**: 本文档是 xylitol 项目的通用规范文档。

---

## 当前阶段: 代码审计与架构优化 (2026-06-11)

**功能开发已冻结。** 当前重点工作: 审计现有代码、修复 clippy warnings、优化架构。
不新增任何功能。详见 `_NEXT.md`。

## 明确不做的功能 (permanent)

| 类别 | 原因 |
|------|------|
| PackageManager 检测 (npm/pnpm/yarn/bun) | 用户自行管理依赖 |
| OAuth / auth-storage / token 管理 | 用户自行配置 API key |
| Extensions SDK / 插件系统 | 不实现 |
| 额外 LLM provider (Gemini, Ollama 等) | 仅 OpenAI-like + Anthropic-like |
| 多模态输入 (图片/文档) | 不规划 |
| TUI / GUI / Web 界面 | 仅 CLI 单次模式; 交互形态待定 |

## Provider 策略

仅支持两种 API 接口:
- **OpenAI-compatible API** — 兼容 OpenAI chat completions 的任意端点（用户自配 `base_url` + `api_key`）
- **Anthropic-compatible API** — 兼容 Anthropic messages 的任意端点（用户自配 `base_url` + `api_key`）

无内置模型列表、无自动发现、无 OAuth 流程。用户自行提供 API key 和 endpoint。

## 交互形态策略

当前: **CLI 单次模式** (`print` 模式)。agent 接收任务 → 执行 → 退出。
未来的交互形态 (TUI / GUI / Web / MCP server) **待定** — 未决策前不实现。

---

## 项目简介

xylitol 是一个 Rust 编写的 AI coding agent。单 crate + 领域分层架构。自主实现，从零构建。

---

## 架构

```
src/
├── agent/       # 核心: agent loop, session, trust, tools, provider, registry, resolver, prompts
├── infra/       # 基础设施: hooks, skills, session (compaction/storage/manager), config, resource
└── interface/   # 入口: cli, print, acp, diff_review
```

层间通过 `pub(crate)` 控制可见性，跨层访问通过 `lib.rs` re-export。

## Feature Flags

命名规则：`<domain>-<capability>`

- **核心**（始终编译）：Agent 循环, 7 内置工具, LLM Provider, Config, CLI + Print 模式
- **内置增强**（始终编译，运行时 config 开关）：Hook 系统, 安全策略, 重复检测
- **可选扩展**（Cargo feature flag）：

| Feature | 层 | 说明 |
|---------|-----|------|
| `infra-skills` | infra | MCP skills |
| `infra-session` | infra | Session 持久化 (JSONL, tree, fork) |
| `ui-review` | interface | Diff review 渲染 |

**默认 features**: `infra-skills`, `infra-session`, `ui-review`

> 以下 feature flags 通过 Cargo.toml 显式 opt-in（不在默认编译中）: `agent-planning`, `agent-model-lock`, `infra-lsp`, `infra-dap`, `infra-acp`, `infra-sandbox`, `infra-rtk`, `dev-vt100`, `dev-e2e`, `dev-fake-provider`

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
- **功能开发已冻结。** 当前阶段不再新增 proposal。仅用于审计/优化相关的追踪变更。

---

## 参考

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Conventional Commits](https://www.conventionalcommits.org/)
