<!-- LLMANSPEC:START -->
# LLMAN 规范驱动开发

本项目使用 llman SDD。阅读 `llmanspec/config.yaml` 了解项目上下文与规则。

使用 `/llman-sdd-onboard` 开始，然后使用 `/llman-sdd-*` 技能进行工作流。

保留此托管块，便于 `llman sdd update` 刷新。
<!-- LLMANSPEC:END -->

# AGENTS.md

> **SSOT**: 本文档是 xylitol 项目的通用规范文档。所有开发指南都应引用本文档，避免重复。

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
- 需要全局 DAG 可视化时运行：`./scripts/generate-llmanspec-changes-dag.sh`（生成 `llmanspec/dag.gen.md`，不入库）

---

## 参考

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Conventional Commits](https://www.conventionalcommits.org/)
