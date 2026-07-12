<!-- LLMANSPEC:START -->
# LLMAN 规范驱动开发

本项目使用 llman SDD。阅读 `llmanspec/config.yaml` 了解项目上下文与规则。

使用 `/llman-sdd-explore` 开始，然后按照 pipeline：`/llman-sdd-propose` → `/llman-sdd-apply` → `/llman-sdd-verify` → `/llman-sdd-archive`。

保留此托管块，便于 `llman sdd update` 刷新。
<!-- LLMANSPEC:END -->

# 仓库级 Agent 指南

用与用户相同的语言回复。`xylitol` 是 Rust 2024 单 crate 的 **开箱即用个人 coding agent**（LLM 增强开发工具包），用 llman SDD 驱动开发。

## 产品定调

| 是 | 不是 |
|---|---|
| 个人默认就能用的 coding harness | 扩展市场 / Extension / 插件平台 |
| 合理分层 + 少量稳定库入口（开闭） | 为「未来插件」堆抽象 |
| 按本仓库需求演进 | 持续对齐 / 追平 `../pi`（`xylitol-tui` 为源自 pi-tui 的独立 fork，见该包 `NOTICE`） |

- **Trust**：对齐 pi 语义——闸的是**项目本地资源是否加载**（settings / prompts / skills / …），不是工具调用 popup。工具侧开箱 **allow-all**。
- **MCP**：配置驱动；未配置则零装配（zero-cost）；支持动态配置与重载。细节见 `src/AGENTS.md`。
- **TUI 产品面**：已开闸；c465 bridge 已归档；下一实现入口 `c475-add-app-tui-chrome` / `c480-add-app-tui-input`（见 `src/app/tui/AGENTS.md`）。c491 假树 stub 仍禁止在其上扩活树。
- 短索引与临时交接：`_HANDOFF.md`（`_NOTE.md` 仅为跳转 stub）；高维产品/业务图：`docs/architecture/`（入口 `docs/architecture/README.md`）。

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
| `domain/` | 纯领域词汇（`XyEvent` / 消息类型等） | 零 crate 内依赖 |
| `runtime_protocol/` | agent↔infra 边界 traits（ports） | 只依赖 `domain/` |
| `agent/` | 薄编排核心（ReAct 循环、session、model、tools 聚合） | 不依赖 `infra`（arch_guard 强制） |
| `infra/` | 运行时域（provider adapter、工具实现、config、session 等） | 不依赖 `agent`（arch_guard 强制） |
| `protocol/` | client↔core 线协议 SSOT（`Command`/`Event`） | 传输无关，只依赖 `domain/` |
| `app/` | 应用面（`cli` print / `server` / `tui`）+ 跨面 seam（`core/`） | 走 seam 不 reach 内部，见 `src/app/AGENTS.md` |
| `packages/xylitol-tui` | 通用 TUI 引擎与组件库（workspace 包） | 零引用主 crate；见该包 `AGENTS.md` |

依赖方向：`app → agent → runtime_protocol → domain`，`infra → runtime_protocol → domain`，`protocol → domain`。

## `Xy*` 与外部库包装

- **`Xy*`** = 近期要精选进库 `pub use` 的**跨层契约 / 可替换端口 / 跨面事件**（方便导出），不是全局品牌前缀。细则 SSOT：`src/AGENTS.md`。
- **外部库概念**：会出现在库入口或多方言统一处 → 包一层（我们的类型）；纯内部实现细节 → **直接用** crate 类型，不 newtype。
- 应用面缝（`Driver` / bootstrap / dispatch）与内部协作者**不加** `Xy`。

## 编码规则

- 遵循 `rustfmt.toml`：Rust 2024、行宽 100、4 空格、field init shorthand、`?` 简写。TOML/YAML 用 2 空格（`.editorconfig`）。
- snake_case 用于模块/文件/函数/变量，PascalCase 用于类型与 trait。
- 模块边界贴合 `src/AGENTS.md` 的分层。
- 单行能内联就别套 wrapper；一两行函数不两层封装。
- 不加向后兼容 shim，除非明确要求；一次性改完旧调用点与格式。
- 格式与 lint 交给工具（`just fmt` / `just lint`），不要在 AGENTS 里重复空格/换行细则。

## Provider 支持范围（Pre-1.0.0）

**交付**：只支持两类 provider API——**OpenAI 兼容**（Chat Completions）与 **Anthropic**（Messages）。OAuth、专属 attribution header、其它厂商专属逻辑在 1.0.0 前不支持；用户自定义仅当声明为上述兼容 API 时接受。

**抽象**：业务只依赖 `XyModel`；多方言收在 infra 适配器族（开闭：新厂商 = 新适配器，不改 ReAct / 应用面）。禁止 Completions「已是 `XyModel` 再包一层」双路径。

## 命令

`just setup`（prek hooks）、`just fmt`、`just lint`（clippy）、`just test`（nextest 或 cargo test）、`just test-tui`（包 TUI 层 1–4）、`just qa`/`just ci`（**统一满闸**：fmt+clippy+test+test-tui+docs+DESIGN tokens+prek）、`just qa-e2e`（`qa` + PTY/tmux 第 5 层，按需）。本地探查 `cargo run -- --help`。API 文档 `cargo doc --no-deps --all-features`。

## 提交与测试

- 提交用 Conventional Commits：`feat(cli): …`/`fix(agent): …`/`refactor(config): …`/`docs: …`/`chore: …`。开 PR 前跑 **`just qa`**；真终端协议/渲染再跑 **`just qa-e2e`**（或 `just test-tui-e2e`）。
- BDD 场景在 `tests/features/*.feature`，rstest-bdd 实现在 `tests/bdd.rs`；需顺序/共享状态时 `cargo test bdd -- --test-threads=1`。快照用 `insta`，接受前复核。回归放 `tests/regression/{issue号}-{简述}.rs`。优先扩既有测试文件，别为小特性新建。
- **测试分层**：BDD 覆盖端到端编排（agent 循环、工具完整路径、session、CLI slash、跨组件）；`#[cfg(test)]` 覆盖纯数据/算法/组件内状态机。已有 BDD 的路径，单测只测底层边界，不重复全链路。
- 实现计划变更后同步 `llmanspec/` 工件（`/llman-sdd-*` 技能）。读代码优先 `rg`。

## llmanspec 命名

- 目录名 = capability：领域名词、kebab-case；**purpose / statement / scenario 强制中文**（标识符可英文）。
- 前缀（按层）：
  - `package-tui-*` — `packages/xylitol-tui`
  - `app-tui-*` — `src/app/tui`（如 `app-tui-host`）；单体 `app-tui` 正退役（c450）
  - 其它意向：`domain-*` / `runtime-*` / `agent-*` / `infra-*` / `protocol-*` / `cli-*` / `server-*` / `test-*`（P0+P1 rename 已完成；`architecture` 等 meta 名仍待议）
- 细则 SSOT：`llmanspec/config.yaml` → `rules.proposal`。

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
