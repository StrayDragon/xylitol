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

- **Trust**：闸的是**项目本地资源是否加载**，不是工具调用 popup。工具侧开箱 **allow-all**。
- **MCP**：配置驱动；未配置则零装配；支持动态重载。实现边界见 `src/AGENTS.md`。
- **产品面**：Print + TUI（TTY 默认）已开闸；走 `XyDriver` + `XyEvent`。TUI 专属规则见 `src/app/tui/AGENTS.md`。
- 产品心智图：`docs/architecture/`；候补方向：`docs/roadmaps/`（不维护进度列）；文档闭环：`docs/AGENTS.md`。

## 工作原则

- 第一性原理：真实需求、代码事实、验证结果；目标不清先对齐。
- **代码是真值源**；架构规则 SSOT 在 `src/AGENTS.md`；设计史在 `llmanspec/changes/archive/`。
- 动代码前读相关代码，沿目录树遵循最近的 `AGENTS.md`。
- 改动聚焦；提交不加 co-author / 不暴露 agent 身份。

## 分层地图（摘要）

单 crate 逻辑分层（**不**拆主 crate）。依赖与 seam 纪律 **只**在 `src/AGENTS.md`；此处一行角色：

| 层 | 角色 |
|---|---|
| `protocol/` | wire + ports + 根上共享类型 |
| `agent/` | 薄编排（ReAct、session、投影） |
| `infra/` | ports 实现与 vendor |
| `app/` | 应用面 + `core` seam |
| `packages/xylitol-tui` | 通用 TUI 引擎（零引用主 crate） |

`app → agent|infra → protocol`；`agent` ↛ `infra`；`infra` ↛ `agent`。

## `Xy*` 与外部库

- **`Xy*`** = 库入口级跨层契约 / 端口 / 跨面事件（精选 `pub use`），不是全局前缀。细则：`src/AGENTS.md`。
- 会出现在库入口或多方言统一 → 包一层；纯内部 → 直接用上游类型。
- 应用缝里的装配细节与内部协作者不加 `Xy`（`XyDriver` 本身是共享应用协议，例外见 `src/AGENTS.md`）。

## 编码规则

- 遵循 `rustfmt.toml` / `.editorconfig`；命名：snake_case 模块与函数，PascalCase 类型。
- 模块边界贴合分层；能内联不套 wrapper；不加无要求的兼容 shim。
- 格式与 lint 交给 `just fmt` / `just lint`，勿在 AGENTS 重复排版细则。

## Provider（Pre-1.0.0）

**交付**：仅 OpenAI 兼容（Chat Completions / Responses）与 Anthropic Messages。OAuth、厂商专属 attribution 等 1.0 前不做。

**开闭**：业务只依赖 `XyModel`；方言在 `xylitol-ai-bridge`。新兼容端 = adapter/配置，**不改** ReAct / `AgentMessage`。禁止「已是 `XyModel` 再包一层」。消息投影细则：`src/AGENTS.md`。

## 命令与验证

- 日常：`just setup` / `fmt` / `lint` / `test` / `test-tui`；满闸 `just qa`（或 `ci`）；真终端协议再 `just qa-e2e`。
- 闸默认 quiet；`just qa normal` / `verbose` 或 `JUST_VERBOSITY=`。
- 非变更闸脚本 `scripts/check_*.py` **MUST** 经 wiring 进 `qa`；维护脚本不进闸。
- 探查：`cargo run -- --help`；文档：`cargo doc --no-deps --all-features`。

## 提交与测试

- Conventional Commits；开 PR 前 `just qa`。
- BDD-on / Partitioned SSOT：live specs + features；流程见 `llmanspec/AGENTS.md`。**禁止** `solidify` / `change delta` / 新建 `*.feature.delta.toon`。
- 测试分层：BDD 管端到端编排；单测管纯数据/组件边界，不重复全链路。
- 实现计划变更后同步 `llmanspec/`；读代码优先 `rg`。

## llmanspec 命名

- capability = 领域名词、kebab-case；purpose / statement / scenario **中文**。
- 前缀按层：`package-tui-*` / `app-tui-*` / `agent-*` / `infra-*` / `protocol-*` / `cli-*` / `server-*` / `test-*` 等。细则：`llmanspec/config.yaml` → `rules.proposal` 与 `llmanspec/AGENTS.md`。

## Skills

SDD：`llman-sdd-*`。应用面：`write-surface`、`audit-dead-code`、`write-tui`。TUI 验证：`test-tui-harness`。观测窄读：`xylitol-inspect-runtime-logs`。

## 编写与维护 AGENTS.md

`AGENTS.md` = **稳定操作边界**，不是 README / 进度板。

| 写 | 不写 |
|---|---|
| WHAT、硬约束、依赖方向、开闭规则 | 进度、行数快照、commit 列表、待办板 |
| HOW 指针（skill / 命令 / `docs/architecture`） | 长教程、完整测试矩阵、易腐文件地图 |
| 与父级不同的本地例外 | 重复父级长文；钉死易变文件路径 / change id |

**偏好**：全局规则与架构不变量；落点用层名（`agent`/`infra`/…），细节去读代码。子目录只补本面例外。

放哪：全仓规则 → 根；目录边界 → 该目录 `AGENTS.md`；多步 how-to → skill；临时笔记 → `_HANDOFF` / `*.tmp.md`（勿升格规范）。

维护：六个月后是否仍真？否则不要进 AGENTS。体量软硬顶与拆分默认策略在 `src/AGENTS.md`；**不**另维护超标表。
