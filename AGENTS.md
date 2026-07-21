<!-- LLMANSPEC:START -->
# LLMAN 规范驱动开发

本项目使用 llman SDD。阅读 `llmanspec/config.yaml` 了解 SDD 命令行为配置，以及 `llmanspec/AGENTS.md` 获取项目附加规则。

## SDD 流水线

使用 `/llman-sdd-explore` 开始，然后按照 pipeline：`/llman-sdd-propose` → `/llman-sdd-apply` → `/llman-sdd-verify` → `/llman-sdd-archive`。

保留此托管块，便于 `llman sdd init --update` 刷新。
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
- **TUI 产品面**：已开闸；c465 bridge 已归档；下一实现入口见 `src/app/tui/AGENTS.md`（`layout/`+`widgets/` 已落地；历史 change `c475-add-app-tui-chrome` / `c480-add-app-tui-input`）。c491 假树 stub 仍禁止在其上扩活树。
- 高维产品/业务图：`docs/architecture/`（入口 `docs/architecture/README.md`）；统一候补方向：`docs/roadmaps/`（不维护进度状态列）；产品文档闭环规则：`docs/AGENTS.md`（roadmaps → llmanspec → architecture）。

## 工作原则

- 从第一性原理出发，先看真实需求、代码事实、验证结果；目标不清先和用户对齐。
- 代码是真值源，不是文档。分层与历史的真值在 `src/AGENTS.md` 与 `llmanspec/changes/archive/<变更>/design.md`。
- 动代码前读相关代码，沿目录树遵循最近的 `AGENTS.md`。
- 改动聚焦，不夹带无关重构。
- 提交不加 co-author 归因，不在 commit/PR/说明里暴露 agent 身份。

## 项目结构（分层地图）

单 crate 分层（**不**拆 crate，避免编译产物膨胀）。跨层纪律写在 `src/AGENTS.md`，靠 review + 缝/行为测试守住——**不用**源码 grep 元测试卡 import 路径。下表只给一行角色；细节以 `src/AGENTS.md` 为准。

| 层 | 角色 | 关键约束 |
|---|---|---|
| `protocol/` | `wire/`（Command/Event）+ `ports/`（XyModel/XyTool/…）+ 根上共享类型（AgentMessage/`XyEvent`/…） | MUST NOT 依赖 agent/infra；MAY 依赖 bridge **DTO only** |
| `agent/` | 薄编排核心（ReAct、session、model、`project_for_llm`） | 不依赖 `infra`（约定 + review） |
| `infra/` | 运行时域（provider adapter、工具实现、config、session 等） | 不依赖 `agent`（约定 + review） |
| `app/` | 应用面（`cli` print / `server` / `tui`）+ 跨面 seam（`core/`） | 走 seam 不 reach 内部，见 `src/app/AGENTS.md` |
| `packages/xylitol-tui` | 通用 TUI 引擎与组件库（workspace 包） | 零引用主 crate；见该包 `AGENTS.md` |

依赖方向：`app → agent → protocol`，`app → infra → protocol`；`agent` ↛ `infra`；`infra` ↛ `agent`。

## `Xy*` 与外部库包装

- **`Xy*`** = 近期要精选进库 `pub use` 的**跨层契约 / 可替换端口 / 跨面事件**（方便导出），不是全局品牌前缀。细则 SSOT：`src/AGENTS.md`。
- **外部库概念**：会出现在库入口或多方言统一处 → 包一层（我们的类型）；纯内部实现细节 → **直接用** crate 类型，不 newtype。
- 应用面缝（`XyDriver` / bootstrap / dispatch）与内部协作者**不加** `Xy`。

## 编码规则

- 遵循 `rustfmt.toml`：Rust 2024、行宽 100、4 空格、field init shorthand、`?` 简写。TOML/YAML 用 2 空格（`.editorconfig`）。
- snake_case 用于模块/文件/函数/变量，PascalCase 用于类型与 trait。
- 模块边界贴合 `src/AGENTS.md` 的分层。
- 单行能内联就别套 wrapper；一两行函数不两层封装。
- 不加向后兼容 shim，除非明确要求；一次性改完旧调用点与格式。
- 格式与 lint 交给工具（`just fmt` / `just lint`），不要在 AGENTS 里重复空格/换行细则。

## Provider 支持范围（Pre-1.0.0）

**交付**：只支持两类 provider API——**OpenAI 兼容**（Chat Completions / Responses）与 **Anthropic**（Messages）。OAuth、专属 attribution header、其它厂商专属逻辑在 1.0.0 前不支持；用户自定义仅当声明为上述兼容 API 时接受。

**抽象（开闭）**：业务只依赖 `XyModel`；方言差异收在 `packages/xylitol-ai-bridge` adapter 族。**优先厂商官方 SDK Client**（兼容端靠 base_url/配置；Responses 流式宽松解析见该包 `AGENTS.md`）。新兼容供应商 = 新 adapter/配置，**不改** ReAct / `AgentMessage`。禁止 Completions「已是 `XyModel` 再包一层」双路径。

**消息分层**：`AgentMessage` = `Llm(LlmMessage) | Env(…)`；发模型前 `project_for_llm → Vec<LlmMessage>` 再映射 bridge DTO。细则：`src/AGENTS.md`「Provider 适配」、`packages/xylitol-ai-bridge/AGENTS.md`。

## 命令

`just setup`（prek hooks）、`just fmt`、`just lint`（clippy）、`just test`（nextest 或 cargo test）、`just test-tui`（包 TUI 层 1–4）、`just qa`/`just ci`（**统一满闸**：fmt+clippy+test+test-tui+docs+DESIGN tokens+`scripts/check_*` 入闸校验与执行+prek）、`just qa-e2e`（`qa` + PTY/tmux 第 5 层，按需）。闸默认 `verbosity=quiet`（成功零输出、失败打全量日志；`just qa normal` / `verbose` 或 `JUST_VERBOSITY=`）。`scripts/check_*.py`（或 `check-*.py`）= 非变更闸脚本，**MUST** 经 `check-scripts-wired`/`check-scripts` 进 `qa`；`cleanup_*` 等维护脚本不进 `qa`。本地探查 `cargo run -- --help`。API 文档 `cargo doc --no-deps --all-features`。

## 提交与测试

- 提交用 Conventional Commits：`feat(cli): …`/`fix(agent): …`/`refactor(config): …`/`docs: …`/`chore: …`。开 PR 前跑 **`just qa`**；真终端协议/渲染再跑 **`just qa-e2e`**（或 `just test-tui-e2e`）。
- BDD（BDD-on / Partitioned SSOT）：手写场景可在 `tests/features/*.feature`；可执行合约场景在 live `llmanspec/specs/<cap>/<cap>.feature`（`@req:` + 场景标题 = `scenario.id`），与 `spec.toon` 约束并存。工作流：feature 分支上改 live specs → `llman sdd change attach` → 实现/校验 → 优先 `change finalize`（单 commit；或 fallback 干净树 `checkpoint` → docs-only `archive`）→ Git merge。**禁止** `solidify` / `change delta` / 新建 `*.feature.delta.toon`。细则见 `llmanspec/AGENTS.md`。step 在 `tests/bdd.rs`；需顺序时 `cargo test --test bdd -- --test-threads=1`。快照用 `insta`。回归放 `tests/regression/{issue号}-{简述}.rs`。
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

SDD：`.agents/skills/llman-sdd-*`。应用面：`write-surface`、`audit-dead-code`、`write-tui`。TUI 五层自动验证：`test-tui-harness`。运行时 log/trace 窄读：`xylitol-inspect-runtime-logs`。

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
- 新增规则要有代码或可验证行为支撑；删过时规则，避免沉默腐烂。
- 子文件变长时拆 skill，不要把根或子 AGENTS 写成百科。
- **体量**：生产模块避免无结构 God 文件；软顶/硬顶与拆分默认策略见 [`src/AGENTS.md`](src/AGENTS.md)「体量与拆分策略」。**不**另维护易腐超标表 / `_TODO` 进度板。
