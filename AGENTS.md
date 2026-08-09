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
- **跨面公共体验**：TUI 与未来 Web **共有**能力的动作语义 / 学习成本 MUST 同源；快捷键 SHOULD 尽量同构；仅面专属能力可分叉。约束板：`docs/roadmaps/Web与TUI同源.md`。
- 产品心智图：`docs/architecture/`；候补方向：`docs/roadmaps/`（不维护进度列）；文档闭环：`docs/AGENTS.md`。
- TUI chrome / 滚动区固定词（下轮预告、滚动提示、尾随…）：`docs/architecture/TUI信息面与chrome词汇.md`（面约束见 `src/app/tui/AGENTS.md`）。

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
| `agent/` | 薄编排（ReAct、capabilities、投影） |
| `infra/` | ports 实现与 vendor |
| `app/` | 应用面 + `core` seam |
| `packages/xylitol-tui` | 通用 TUI 引擎（零引用主 crate） |

`app → agent|infra → protocol`；`agent` ↛ `infra`；`infra` ↛ `agent`。

## `Xy*` 与外部库

- **`Xy*`** = 库入口级跨层契约 / 端口 / 跨面事件（精选 `pub use`），不是全局前缀。哪些加、哪些不加：`src/AGENTS.md`。
- 会出现在库入口或多方言统一 → 包一层；纯内部 → 直接用上游类型。

## 编码规则

- 遵循 `rustfmt.toml` / `.editorconfig`；命名：snake_case 模块与函数，PascalCase 类型。
- 模块边界贴合分层；能内联不套 wrapper；不加无要求的兼容 shim。
- 格式与 lint 交给 `just fmt` / `just lint`，勿在 AGENTS 重复排版细则。

## Provider（Pre-1.0.0）

**交付**：仅 OpenAI 兼容（Chat Completions / Responses）与 Anthropic Messages。OAuth 等 1.0 前不做。OpenCode Zen（`opencode.ai`）的 `x-opencode-*` 会话归因属于网关 courtesy，允许。

**开闭**：业务只依赖 `XyModel`；方言在 `xylitol-ai-bridge`。新兼容端 = adapter/配置，**不改** ReAct / `AgentMessage`。禁止「已是 `XyModel` 再包一层」。消息投影细则：`src/AGENTS.md`。

## 命令与验证

- 日常：`just setup` / `fmt` / `lint` / `test` / `test-tui`；满闸 `just qa`（或 `ci`）；真终端协议再 `just qa-e2e`。
- `just qa` 在 workspace 测试之后串行跑 `test-live-provider`（`--test-threads=1`，不进 nextest 并行矩阵）；专用配置 `<global-dir>/dev/live-provider.yaml`（全局目录：`$XYLITOL_CONFIG_DIR` → `$XDG_CONFIG_HOME/xylitol` → `~/.config/xylitol`，经 dotxylitol 多机共享；示例由 `just gen-live-provider-example` 自动生成；`enabled: true` 才实打网关；缺失/关闭则 skip）。产品示例配置 `configs/example.yaml` 由 `just gen-config-example` 生成（改 `scripts/gen_config_example.py` 模板，勿直接手改产物）。
- 闸默认 quiet；`just qa normal` / `verbose` 或 `JUST_VERBOSITY=`。
- 非变更闸脚本 `scripts/check_*.py` **MUST** 经 wiring 进 `qa`；维护脚本不进闸。
- 探查：`cargo run -- --help`；文档：`cargo doc --no-deps --all-features`。

## Worktree 并行开发

多 worktree 并行（多个 SDD change / feature 分支同时开）时遵循：

- **硬规则**：同 crate 的不同 worktree **禁止**共用 `CARGO_TARGET_DIR`。Cargo fingerprint 不区分包绝对路径，可错误标 `Fresh` 并跑到别的树编出的二进制（1.97.x 已复现）。
- 每树独立 target：
  ```bash
  eval "$(just cargo-wt-env)"        # 或 source scripts/cargo_worktree_env.sh
  ```
  输出 `CARGO_TARGET_DIR=~/.cache/cargo-targets/<repo>/<wt-key>/`，按仓库根路径哈希隔离；脚本见 `scripts/cargo_worktree_env.sh`（维护脚本，不进 qa）。
- **共享层**：`~/.cargo`（registry/git）+ sccache（`RUSTC_WRAPPER=sccache`）跨树安全；`SCCACHE_CACHE_SIZE` 防止缓存反噬磁盘。
- 清盘：删除旧 worktree 后顺手 `rm -rf ~/.cache/cargo-targets/<repo>/<对应key>/`；大 target 内部结构（debuginfo / incremental）处置见 `docs/research/rust-disk-worktree-cache-2026.md` 与 skill `rust-build-tune`。

## 提交与测试

- Conventional Commits；开 PR 前 `just qa`。
- BDD-on / Partitioned SSOT：live specs + features；流程见 `llmanspec/AGENTS.md`。**禁止** `solidify` / `change delta` / 新建 `*.feature.delta.toon`。
- 测试分层：BDD 管端到端编排；单测管纯数据/组件边界，不重复全链路。
- 实现计划变更后同步 `llmanspec/`；读代码优先 `rg`。

## llmanspec 命名

- capability = 领域名词、kebab-case；purpose / statement / scenario **中文**。
- 前缀按层：`package-tui-*` / `app-tui-*` / `agent-*` / `infra-*` / `protocol-*` / `cli-*` / `server-*` / `test-*` 等。细则：`llmanspec/config.yaml` → `rules.proposal` 与 `llmanspec/AGENTS.md`。

## Specs 约束层级（产品级优先）

- specs 的 requirement 约束落在**产品级**：可观察行为、产品语义、数据契约、验证结果；**禁止硬约束代码组织**——具体路径、文件/模块名、类型名、行数、方法归属、迁移清单（除非是**大的组织方向**：分层依赖、端口 seam、crate 边界、组合根职责、跨面同源）。
- 代码组织演进（重构、改名、移动）不要求改 spec；spec 只随产品行为变化而变。代码是真值源，spec 不追平组织细节。
- 已删除对象（类型/模块/方法）的引用条款随删除一并清理，不保留「防复活」清单（除非真实回归风险）。
- 细则：`llmanspec/AGENTS.md` → 「spec 约束层级」；组织细节维护落在 `src/AGENTS.md`，不落在 specs。

## Skills

SDD：`llman-sdd-*`（含 `llman-sdd-quick` 快速路径）。应用面：`write-surface`、`audit-dead-code`、`write-tui`。TUI 验证：`test-tui-harness`。观测窄读：`xylitol-inspect-runtime-logs`。构建/磁盘：`rust-build-tune`。TUI 参考：`tui-expert-of-codex`、`terminal-tui-differential-rendering`。

## 编写与维护 AGENTS.md

`AGENTS.md` = **稳定操作边界**，不是 README / 进度板。

| 写 | 不写 |
|---|---|
| WHAT、硬约束、依赖方向、开闭规则 | 进度、行数快照、commit 列表、待办板 |
| HOW 指针（skill / 命令 / `docs/architecture`） | 长教程、完整测试矩阵、易腐文件地图 |
| 与父级不同的本地例外 | 重复父级长文；钉死易变文件路径 / change id |

**偏好**：全局规则与架构不变量；落点用层名（`agent`/`infra`/…），细节去读代码。子目录只补本面例外。

放哪：全仓规则 → 根；目录边界 → 该目录 `AGENTS.md`；多步 how-to → skill；临时笔记 → `_HANDOFF` / `*.tmp.md`（勿升格规范）；**change 作用域调研** → `llmanspec/changes/<id>/research/`（细则 `llmanspec/AGENTS.md`，勿堆 `docs/research/`）。

维护：六个月后是否仍真？否则不要进 AGENTS。体量软硬顶与拆分默认策略在 `src/AGENTS.md`；**不**另维护超标表。
