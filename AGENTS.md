# AGENTS.md

Project conventions for xylitol.

> **SSOT (Single Source of Truth)**: 本文档是 xylitol 项目的通用规范文档。所有开发指南、文档都应引用本文档，避免重复和不一致。

---

## Rust

### Edition & Toolchain

- Edition 2024
- Stable toolchain (pinned in `rust-toolchain.toml`)

### Code Quality

- Clippy thresholds configured in `.clippy.toml`; lint flags in justfile
- Format config in `rustfmt.toml`
- Nightly-only options are not allowed

### 命名约定

| 类型 | 约定 | 示例 |
|------|------|------|
| Struct | `PascalCase` | `Config` |
| Enum | `PascalCase` | `Error` |
| Function | `snake_case` | `new`, `from_str` |
| Const | `SCREAMING_SNAKE_CASE` | `MAX_SIZE` |
| Trait | `PascalCase` | `Transport` |

### 错误处理

- 使用 `thiserror` 定义错误枚举
- 避免使用 `.unwrap()` 和 `.expect()`（除测试代码）

### 异步代码

- 使用 `tokio` 作为异步运行时
- 使用 `async-trait` 定义异步 trait
- 所有 I/O 操作基于异步

### 日志规范

- 使用 `tracing` 库（非 `log`）
- 结构化日志，使用 `info!`, `debug!`, `error!`, `warn!`

---

## Git Hooks (`prek.toml`)

- Only `repo = "builtin"` + `repo = "local"` — zero network dependency at runtime
- Local hooks: `pass_filenames = false` + `require_serial = true` (avoid cargo concurrent conflicts)
- Stages:
  - **pre-commit**: cargo-fmt, cargo-clippy (+ builtin whitespace/toml/yaml checks)
  - **commit-msg**: conventional commits validation
  - **pre-push**: cargo-test (full tests, not on every commit)

---

## Commits

- Conventional Commits format: `type(scope)!: description`
- Types: feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert
- Subject line max 72 characters

---

## Task Runner (`justfile`)

- `just setup` = 安装 prek hooks
- `just fmt` = 格式化代码（写入）
- `just fmt-check` = 检查格式（只读）
- `just lint` = 运行 clippy
- `just test` = 运行测试
- `just qa` = fmt-check + lint + test + doc-check（所有检查）
- `just check` = qa 的别名
- `just ci` = prek run --all-files + qa（完整 CI 模拟）
- Shell: `bash -euo pipefail`

---

## 文档规范

### Markdown

- 代码块指定语言：\`\`\`rust, \`\`\`bash
- 链接使用相对路径（内部文档）

### 代码注释

- 公共 API 必须有文档注释（`///` 或 `//!`）
- 模块级文档使用 `//!`
- `#![cfg_attr(docsrs, warn(missing_docs))]` — docs.rs 构建时未文档化公开项产生警告

### 文档命令

```bash
just doc          # 构建 API 文档并在浏览器打开
just doc-check    # 检查 API 文档构建是否成功 (CI 使用)
just doc-test     # 运行文档测试 (验证 /// 示例编译)
```

---

## Scripts (`scripts/`)

- **Purpose**: 只用于复杂的、项目特定的逻辑
- **Naming**: kebab-case
- **避免**: 简单的 cargo 命令包装（直接在 justfile 中调用）
- **No underscores** in filenames — prevents accidental Python module import

---

## 测试规范

### 测试组织

- 单元测试：与源码同目录的 `tests` 模块
- 集成测试：`tests/` 目录
- 性能测试：`benches/` 目录（使用 criterion）

### 测试命名

- 测试函数：`test_<功能>_<场景>`
- 测试模块：`tests` 或 `<module>_tests`

---

## 构建配置

### Cargo.toml Profiles

| Profile | 目标 | 关键配置 |
|---------|------|----------|
| dev | 编译速度优先 | `opt-level = 0`, `codegen-units = 256`, 依赖 `opt-level = 3` |
| release | 体积+性能优先 | `opt-level = "z"`, `codegen-units = 1`, `lto = "thin"`, `strip = "symbols"`, `panic = "abort"` |

### 异步超时与 CI 可靠性

| 模式 | 规则 |
|------|------|
| 异步循环 | 必须使用 `tokio::time::timeout` 包裹 |
| CI job | 必须设置 `timeout-minutes` |
| 外部服务依赖 | 启动失败 → 优雅跳过，不 panic |

---

## 版本管理

### 语义化版本

- v0.0.0-dev: crate name locking (占位)
- 后续版本按功能里程碑递增

### 检查点规则

每个阶段完成后：验证 (`just qa`) → 提交 (Conventional Commits) → 版本号更新 → 打 tag

### 历史版本管理

- 禁止在打 tag 后修改历史（如有紧急修复，递增 patch 版本）

---

## 参考资源

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [The Rust Style Guide](https://rust-lang.github.io/style-guide/)
- [Conventional Commits](https://www.conventionalcommits.org/)
