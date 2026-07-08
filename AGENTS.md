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

## 项目结构（分层地图）

单 crate 分层架构，跨层依赖由 `src/tests.rs::arch_guard` 强制。分层不变量、各层职责、应用面状态的 SSOT 见 `src/AGENTS.md`；本表只给一行角色 + 关键约束。

| 层 | 角色 | 关键约束 |
|---|---|---|
| `domain/` | 纯领域词汇（`XyEvent`/`XyModel`/`XyTool`/消息类型） | 零 crate 内依赖 |
| `runtime_protocol/` | agent↔infra 边界 traits（ports） | 只依赖 `domain/` |
| `agent/` | 薄编排核心（ReAct 循环、session、model、tools 聚合） | 不依赖 `infra`（arch_guard 强制） |
| `infra/` | 运行时域（provider adapter、工具实现、config、session 等） | 不依赖 `agent`（arch_guard 强制） |
| `protocol/` | client↔core 线协议 SSOT（`Command`/`Event`） | 传输无关，只依赖 `domain/` |
| `app/` | 应用面（`cli` print / `server` / `tui`）+ 跨面 seam（`core/`） | 走 seam 不 reach 内部，见 `src/app/AGENTS.md` |
| `packages/xylitol-tui` | 通用 TUI 引擎与组件库（workspace 包） | 零引用主 crate；见该包 `AGENTS.md` |

依赖方向：`app → agent → runtime_protocol → domain`，`infra → runtime_protocol → domain`，`protocol → domain`。

## 编码规则

- 遵循 `rustfmt.toml`：Rust 2024、行宽 100、4 空格、field init shorthand、`?` 简写。TOML/YAML 用 2 空格（`.editorconfig`）。
- snake_case 用于模块/文件/函数/变量，PascalCase 用于类型与 trait。
- 模块边界贴合 `src/AGENTS.md` 的分层。
- 单行能内联就别套 wrapper；一两行函数不两层封装。
- 不加向后兼容 shim，除非明确要求；一次性改完旧调用点与格式。
- 格式与 lint 交给工具（`just fmt` / `just lint`），不要在 AGENTS 里重复空格/换行细则。

## Provider 支持范围（Pre-1.0.0）

只支持两类 provider API：**OpenAI 兼容**（Chat Completions）与 **Anthropic**（Messages）。其它 provider、OAuth 凭据存储、provider 专属 attribution header 在 1.0.0 前不支持。用户自定义 provider 仅当说 OpenAI/Anthropic 兼容 API 时才接受。给不支持 provider 加专属逻辑的改动，review 时拒绝。

## 命令

`just setup`（prek hooks）、`just fmt`、`just lint`（clippy）、`just test`（nextest 或 cargo test）、`just qa`/`just ci`（fmt+clippy+test+docs+prek）。本地探查 `cargo run -- --help`。API 文档 `cargo doc --no-deps --all-features`。

## 提交与测试

- 提交用 Conventional Commits：`feat(cli): …`/`fix(agent): …`/`refactor(config): …`/`docs: …`/`chore: …`。开 PR 前跑 `just qa`。
- BDD 场景在 `tests/features/*.feature`，rstest-bdd 实现在 `tests/bdd.rs`；需顺序/共享状态时 `cargo test bdd -- --test-threads=1`。快照用 `insta`，接受前复核。回归放 `tests/regression/{issue号}-{简述}.rs`。优先扩既有测试文件，别为小特性新建。
- 实现计划变更后同步 `llmanspec/` 工件（`/llman-sdd-*` 技能）。读代码优先 `rg`。

## llmanspec 命名（workspace 包）

- 主 crate 能力：`llmanspec/specs/<domain-noun>/`（如 `app-tui`、`agent-runtime`）。
- **`packages/xylitol-tui` 能力**：目录与 `name` 字段必须以 `package-tui-` 开头（如 `package-tui-testing`、`package-tui-paste-burst`、`package-tui-editor`）。产品面 TUI 仍用 `app-tui`，不加此前缀。
- 目的：日后若单独分发/迁出 `xylitol-tui`，可按前缀整批迁移 specs，不与主 crate 能力混名。

## Skills

SDD：`.agents/skills/llman-sdd-*`。应用面：`write-surface`、`audit-dead-code`、`write-tui`。TUI 五层自动验证：`test-tui-harness`。

## 编写与维护 AGENTS.md（规范）

`AGENTS.md` 是给智能体的**稳定操作边界**，不是第二份 README，也不是进度板。嵌套：根适用于全仓；子目录只补充本目录专属规则，并覆盖/细化父级中与本目录冲突的部分。同一事实只在一处写全，其它地方用路径指针。

### 写什么 / 不写什么

| 写（短、少变） | 不写（易腐） |
|---|---|
| WHAT：本目录是什么、不是什么 | 进度表、行数、commit 列表、变更日志 |
| 硬约束：依赖方向、禁止 reach、API 边界 | 裁剪清单、待办、预想能力（多 tab 等） |
| HOW 指针：去哪个 skill / 跑哪条命令 | 长 how-to、逐步教程、完整测试矩阵 |
| 与父级不同的本地例外 | 重复根/`src/AGENTS.md` 已有的分层长文 |

### 放哪

1. **几乎所有任务都要遵守** → 根 `AGENTS.md`。
2. **某目录稳定边界** → 该目录 `AGENTS.md`（保持短；改代码边界时才改）。
3. **流程性 how-to / harness / 多步工作流** → `.agents/skills/<name>/SKILL.md`；AGENTS 只留一行指针。
4. **临时交接、复核笔记** → `_HANDOFF.md` / `*.tmp.md` 等，**勿当规范**，勿把清单抄进 AGENTS。

### 维护习惯

- 先问：这条六个月后是否仍真？会否随每个 PR 改？若否 → skill 或 handoff，不是 AGENTS。
- 新增规则要有代码或 arch_guard 事实支撑；删过时规则，避免沉默腐烂。
- 子文件变长时拆 skill，不要把根或子 AGENTS 写成百科。
