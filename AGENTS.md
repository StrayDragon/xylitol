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

xylitol 是一个 Rust 编写的 AI coding agent，参考 codex-rs 设计。采用单 crate + 领域分层 + feature flags 架构。

---

## 架构

```
src/
├── agent/       # 核心领域：agent loop, tools, planner, model
├── infra/       # 基础设施：config, hooks, security, lsp, dap, session, skills
└── interface/   # 用户接口：cli, tui, print, rpc
```

层间通过 `pub(crate)` 控制可见性，跨层访问通过 `lib.rs` re-export。

---

## Feature Flags（两层启用策略）

### 命名规则：`<domain>-<capability>`

| 前缀 | 层 | 含义 |
|------|-----|------|
| `agent-` | `agent/` | 改变 agent 核心行为 |
| `infra-` | `infra/` | 基础设施集成 |
| `ui-` | `interface/` | 用户交互模式 |
| `dev-` | test-only | 开发/测试专用 |

### 两层策略

- **始终编译（built-in）**：无 feature flag，通过 `config.yaml` 运行时控制
- **可选编译（feature-flagged）**：Cargo feature flag 控制，引入重依赖或非必需集成

### 始终编译功能

| 功能 | Config 控制 |
|------|------------|
| 7 个内置工具 | `tools.allowlist` / `tools.blocklist` |
| Hook 事件系统 | `hooks: []`（空 = no-op） |
| 安全策略引擎 | `security.enabled` |
| 重复检测 | `repeat_detection.enabled` |
| Print 模式 | CLI `--mode print` |
| JSON-RPC 模式 | CLI `--mode rpc` |

### 可选编译 Feature Flags

| Feature | 层 | 说明 | 重依赖 |
|---------|-----|------|--------|
| `agent-planning` | agent | 规划-执行分离 | — |
| `agent-model-lock` | agent | 模型抢占锁（Phase 2） | — |
| `infra-lsp` | infra | LSP 集成 | `lspz` |
| `infra-dap` | infra | DAP 集成（Phase 2） | — |
| `infra-skills` | infra | Skills & MCP | `rmcp` |
| `infra-session` | infra | Session 快照 | SQLite |
| `infra-sandbox` | infra | 沙箱（Phase 2） | — |
| `infra-rtk` | infra | rtk 输出压缩 | — |
| `ui-tui` | interface | ratatui TUI | `ratatui` 等 |
| `ui-review` | interface | Diff 评审 | `syntect` |
| `dev-vt100` | dev | VT100 测试 | `vt100` |
| `dev-e2e` | dev | PTY E2E 测试 | — |

**默认**: `ui-tui`, `infra-session`, `ui-review`

---

## Rust

- Edition 2024, stable toolchain (`rust-toolchain.toml`)
- Nightly-only options are not allowed
- `#![cfg_attr(docsrs, warn(missing_docs))]` — 公共 API 必须文档化

### 命名约定

| 类型 | 约定 | 示例 |
|------|------|------|
| Struct / Enum / Trait | `PascalCase` | `Config`, `Error`, `Transport` |
| Function | `snake_case` | `new`, `from_str` |
| Const | `SCREAMING_SNAKE_CASE` | `MAX_SIZE` |

### 错误处理

- `thiserror` 定义错误枚举，`anyhow` 应用层
- 避免 `.unwrap()` / `.expect()`（测试代码除外）

### 异步 & 日志

- `tokio` 运行时，`async-trait` 定义异步 trait，所有 I/O 异步
- `tracing`（非 `log`），结构化日志

---

## Bootstrap & Setup

```bash
# 1. 安装 toolchain
rustup show          # 确认 stable + rustfmt + clippy

# 2. 安装 task runner
cargo install just   # 或系统包管理器

# 3. 安装 git hooks 工具
cargo install prek   # 或系统包管理器

# 4. 安装 hooks
just setup

# 5. 验证环境
just qa

# 6. 开始开发
#    使用 /llman-sdd-onboard 了解工作流
#    使用 /llman-sdd-explore 探索提案
```

---

## Git Hooks (`prek.toml`)

- `repo = "builtin"` + `repo = "local"` only — zero network dependency
- Local hooks: `pass_filenames = false` + `require_serial = true`
- **pre-commit**: cargo-fmt, cargo-clippy (+ builtin checks)
- **commit-msg**: conventional commits validation
- **pre-push**: cargo-test

---

## Commits

Conventional Commits: `type(scope)!: description`，subject max 72 chars。
Types: feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert。

---

## Task Runner (`justfile`)

| Command | Purpose |
|---------|---------|
| `just setup` | 安装 prek hooks |
| `just fmt` | 格式化（写入） |
| `just fmt-check` | 格式化（只读） |
| `just lint` | clippy |
| `just test` | 测试 |
| `just qa` / `just check` | fmt-check + lint + test + doc-check |
| `just ci` | prek run --all-files + qa |

Shell: `bash -euo pipefail`

---

## 测试

- 单元测试：源码同目录 `tests` 模块；集成测试：`tests/`；性能测试：`benches/`
- 命名：`test_<功能>_<场景>`
- 异步循环必须 `tokio::time::timeout` 包裹
- CI job 必须设置 `timeout-minutes`

---

## 构建 Profiles

| Profile | 目标 | 关键配置 |
|---------|------|----------|
| dev | 编译速度 | `opt-level = 0`, 依赖 `opt-level = 3` |
| release | 体积+性能 | `opt-level = "z"`, `lto = "thin"`, `strip = "symbols"`, `panic = "abort"` |

---

## 版本管理

- `v0.0.0-dev`: crate name locking
- 阶段完成：`just qa` → 提交 → 版本号 → tag
- tag 后禁止修改历史（紧急修复递增 patch）

---

## llman SDD 工作流

- 每个 change 的依赖关系声明在 `llmanspec/changes/<id>/proposal.md` 的 YAML frontmatter 中（`depends_on` / `blocks`）
- 处理 change 前必须检查 `depends_on`：未归档的前置 change 未完成则不能开始
- 检查 `blocks`：了解哪些后续 change 依赖当前 change，把握全局影响
- 如果 `depends_on` 引用的 change 不在 `changes/` 目录，可能在 `changes/archive/` 或已被 freeze，告知用户并忽略
- 需要全局 DAG 可视化时运行：`llman sdd graph`

---

## MVP Change DAG

```
c05-init-skeleton ─────────────────────────────────────────────────
  ├─ c10-add-config ───────────────────────────────────────────────
  │   ├─ c15-add-cli ─────────────────────────────────────────────
  │   │   ├─ c30-add-print-mode (built-in)
  │   │   ├─ c80-add-tui (ui-tui)
  │   │   └─ c87-add-rpc-mode (built-in)
  │   ├─ c20-add-tools (built-in) ───────────────────────────────
  │   │   ├─ c25-add-agent-loop ─────────────────────────────────
  │   │   │   ├─ c35-add-repeat-detection (built-in)
  │   │   │   ├─ c55-add-planning-execution (agent-planning)
  │   │   │   │   └─ c60-add-model-lock (agent-model-lock, Phase 2)
  │   │   │   ├─ c70-add-session-snapshot (infra-session)
  │   │   │   ├─ c88-add-test-infra (dev-vt100, dev-e2e)
  │   │   │   └─ (c30, c80, c87 共享依赖)
  │   │   ├─ c40-add-hooks (built-in) ── c50-add-security (built-in)
  │   │   ├─ c65-add-skills-mcp (infra-skills)
  │   │   └─ c75-add-diff-review (ui-review)
  │   ├─ c45-add-lsp-layer (infra-lsp)
  │   └─ c85-add-dap-layer (infra-dap, Phase 2)
```

括号中标注 feature flag 名称或 `built-in`（始终编译）。

---

## 参考

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Conventional Commits](https://www.conventionalcommits.org/)
