<!-- LLMANSPEC:START -->
# LLMAN 规范驱动开发

本项目使用 llman SDD。阅读 `llmanspec/config.yaml` 了解项目上下文与规则。

使用 `/llman-sdd-onboard` 开始，然后使用 `/llman-sdd-*` 技能进行工作流。

保留此托管块，便于 `llman sdd update` 刷新。
<!-- LLMANSPEC:END -->

# 仓库级 Agent 指南

用与用户相同的语言回复。`xylitol` 是 Rust 2024 单 crate 的 LLM 增强开发工具包，用 llman SDD 驱动开发。

## 工作原则

- 从第一性原理出发，先看真实需求、代码事实、验证结果；目标不清先和用户对齐。
- 代码是真值源，不是文档。分层与历史的真值在 `src/AGENTS.md` 与 `llmanspec/changes/archive/<变更>/design.md`。
- 动代码前读相关代码，沿目录树遵循最近的 `AGENTS.md`。
- 改动聚焦，不夹带无关重构。
- 提交不加 co-author 归因，不在 commit/PR/说明里暴露 agent 身份。

## 项目结构

单 crate 分层架构（`domain` → `runtime_protocol` → `agent`/`infra` → `protocol` → `app`），跨层依赖方向由 `src/tests.rs::arch_guard` 强制。分层地图与不变量见 `src/AGENTS.md`；应用面见 `src/app/AGENTS.md`。

## 编码规则

- 遵循 `rustfmt.toml`：Rust 2024、行宽 100、4 空格、field init shorthand、`?` 简写。TOML/YAML 用 2 空格（`.editorconfig`）。
- snake_case 用于模块/文件/函数/变量，PascalCase 用于类型与 trait。
- 模块边界贴合 `src/AGENTS.md` 的分层。
- 单行能内联就别套 wrapper；一两行函数不两层封装。
- 不加向后兼容 shim，除非明确要求；一次性改完旧调用点与格式。

## Provider 支持范围（Pre-1.0.0）

只支持两类 provider API：**OpenAI 兼容**（Chat Completions）与 **Anthropic**（Messages）。其它 provider（Google、DeepSeek、NVIDIA、Groq、Mistral、OpenRouter 等）、OAuth 凭据存储、provider 专属 attribution header 在 1.0.0 前不支持。用户自定义 provider 仅当说 OpenAI/Anthropic 兼容 API 时才接受。给不支持 provider 加专属逻辑的改动，review 时拒绝。

## 命令

`just setup`（prek hooks）、`just fmt`、`just lint`（clippy）、`just test`（nextest 或 cargo test）、`just qa`/`just ci`（fmt+clippy+test+docs+prek）。本地探查 `cargo run -- --help`。API 文档 `cargo doc --no-deps --all-features`。

## 提交与测试

- 提交用 Conventional Commits：`feat(cli): …`/`fix(agent): …`/`refactor(config): …`/`docs: …`/`chore: …`。开 PR 前跑 `just qa`。
- BDD 场景在 `tests/features/*.feature`，rstest-bdd 实现在 `tests/bdd.rs`；需顺序/共享状态时 `cargo test bdd -- --test-threads=1`。快照用 `insta`，接受前复核。回归放 `tests/regression/{issue号}-{简述}.rs`。优先扩既有测试文件，别为小特性新建。
- 实现计划变更后同步 `llmanspec/` 工件（`/llman-sdd-*` 技能）。读代码优先 `rg`。

## Skills

SDD 工作流见 `.agents/skills/llman-sdd-*`。架构与新增面相关：`write-surface`（新增应用面方法论）、`audit-dead-code`（死代码分诊）、`write-tui`（TUI 面改造）。

## 指南更新放哪

- 影响几乎所有任务的硬规则：本根文件。
- 只影响某目录的规则：最近的子目录 `AGENTS.md`（如 `src/AGENTS.md`、`src/app/AGENTS.md`、`src/app/tui/AGENTS.md`）。
- 流程性 how-to：`.agents/skills/<name>/SKILL.md`，并在对应 `AGENTS.md` 引用。
- 更新要聚焦、有代码事实支撑。
