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
- **产品面**：Print + TUI（TTY 默认）已开闸。产品 TUI 默认 attach 本机 Host（`http://127.0.0.1:18790`）；未在听失败。Print 与库嵌入仍可同进程。TUI 专属规则见 `src/app/tui/AGENTS.md`。
- **CS 角色**：TUI / Print 是 **client**（面本地：键、画、TTY、编辑器、剪贴板）。模型 / 会话 / MCP / 工作区执行 / trust 是 **host（操作器角色）**。host **不是**「必须先占端口」——print / 库嵌入仍可无绑定。产品 TUI **要求**监听器。占用绑定的是显式 `xylitol serve`。
- **产品信封（RPC envelope）**：四象限 RPC（unary HTTP POST + WebSocket 下行、不收业务上行）。Command/Event 是载荷，不是外层。类型 SSOT 在 Rust `protocol`（无跨语言类型导出；OpenAPI 仅调试文档）。
- **跨面公共体验**：TUI 与第二产品面（gpui 桌面，Linux/Wayland）**共有**能力的动作语义 / 学习成本 MUST 同源；快捷键 SHOULD 尽量同构；仅面专属能力可分叉。约束板：`docs/roadmaps/Web与TUI同源.md`。
- 产品架构总览：`docs/architecture/`；候选方向：`docs/roadmaps/`（不维护进度列）；文档闭环：`docs/AGENTS.md`。
- TUI 信息面固定词（滚动提示、通知条、尾插…）：`docs/architecture/TUI信息面与固定区词汇.md`（面约束见 `src/app/tui/AGENTS.md`）。
- 交互设计稿：仓库顶层 `designing/`（现 `tui/` + `tui-lab/` 交互实验区，以后可加其它端；`just open-designing`）。**代码是运行时真值**；稿是对照辅助。无独立快捷键设计。UI/UX 迭代走 SOP：候选先进 `tui-lab` 实验原型（staged），评审后晋级产品面或淘汰（见 `designing/AGENTS.md`）。

## 工作原则

- 第一性原理：真实需求、代码事实、验证结果；目标不清先对齐。
- 散文黑话首现须附标准词或英文对照（总表见 `docs/architecture/术语表.md`）；内部编号（`c####` / `ath##` 等）单独引用时须带 ≤12 字语义尾，避免检索死端。
- **代码是真值源**；架构规则 SSOT 在 `src/AGENTS.md`；设计史在 `llmanspec/changes/archive/`。
- 动代码前读相关代码，沿目录树遵循最近的 `AGENTS.md`。
- 改动聚焦；提交不加 co-author / 不暴露 agent 身份。

## 分层地图（摘要）

单 crate 逻辑分层（**不**拆主 crate）。依赖与 seam 纪律 **只**在 `src/AGENTS.md`；此处一行角色：

| 层 | 角色 |
|---|---|
| `protocol/` | wire + ports + 根上共享类型 |
| `agent/` | 薄编排（ReAct、capabilities、投影 projection） |
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
- 优先穷举类型 / 注册表，避免字符串 magics；公共形状让 rust-analyzer 能跳转。

## Pre-0.0.1 卫生（未发布版本）

第一个 tagged **0.0.1** 之前，本仓库 **没有** 外部 SemVer 客户。迭代中 **MUST NOT** 积累「以后再删」的兼容债：

- **禁止**为未发布的公开 API、YAML 键、slash、UI 字符串保留兼容别名、双解析路径、deprecated 转发。改名 = 一次性改调用点。
- Session JSONL：未知字段可忽略（serde）≠ 产品代码永久读旧键。需要读旧会话时做一次性迁移或声明不保证，**禁止**双语义长期并存。
- 死码按 skill `l8ng-audit-dead-code` 分诊（真死删 / 逻辑死本变更内激活或删 / 预留须写落地条件）。禁止无理由新 `#[allow(dead_code)]`。
- **禁止**为未交付能力预留兼容 shim。0.0.1 **之后** 再谈 SemVer / 弃用窗。

## Provider（Pre-1.0.0）

**交付**：仅 OpenAI 兼容（Chat Completions / Responses）与 Anthropic Messages。OAuth 等 1.0 前不做。OpenCode Zen（`opencode.ai`）的 `x-opencode-*` 会话归因属于网关 courtesy，允许。

**开闭**：业务只依赖 `XyModel`；方言在 `xylitol-ai-bridge`。新兼容端 = adapter/配置，**不改** ReAct / `AgentMessage`。禁止「已是 `XyModel` 再包一层」。消息投影细则：`src/AGENTS.md`。

## 命令与验证

- 日常：`just setup` / `fmt` / `lint` / `test` / `test-tui`；全量门禁 `just qa`（或 `ci`）；真终端协议再 `just qa-e2e`。
- `just qa` 在 workspace 测试之后串行跑 `test-live-provider`（`--test-threads=1`，不进 nextest 并行矩阵）；专用配置 `<global-dir>/dev/live-provider.yaml`（全局目录：`$XYLITOL_CONFIG_DIR` → `$XDG_CONFIG_HOME/xylitol` → `~/.config/xylitol`，经 dotxylitol 多机共享；示例由 `just gen-live-provider-example` 自动生成；`enabled: true` 才实打网关；缺失/关闭则 skip）。产品示例配置 `configs/example.yaml` 由 `just gen-config-example` 生成（改 `scripts/gen_config_example.py` 模板，勿直接手改产物）。
- 闸默认 quiet；`just qa normal` / `verbose` 或 `JUST_VERBOSITY=`。
- 非变更闸脚本 `scripts/check_*.py` **MUST** 经 wiring 进 `qa`；维护脚本不进闸。
- 探查：`cargo run -- --help`；文档：`cargo doc --no-deps --all-features`。

### 试验 / 打网命名（`lab_`）

试验性证据与打网探针 **统一前缀 `lab_`**（不再用 `evidence_` / `experiment_` / 试验性 `live_` 二进制名）：

| 形态 | 约定 | 是否进 `just qa` |
|---|---|---|
| `packages/*/examples/lab_*.rs` | 人跑维护 lab（`cargo run -p … --example lab_…`） | **否** |
| `packages/*/tests/lab_*.rs` | 可打真网关的 lab 二进制；是否入闸看 just 接线 | **仅**已接线者（现：`lab_responses_prompt_cache` ← `just test-live-provider`） |
| `#[test] fn lab_*` / `async fn lab_*` | 闸内契约实验（无网或假网） | **是**（随 crate 测） |

配置文件名 `live-provider.yaml` / recipe `test-live-provider` 保留「live 网关」语义，与代码符号前缀无关。新试验 **MUST** 用 `lab_`；禁止再引入 `evidence_` / `experiment_` 前缀。

## Worktree 并行开发

多 worktree 并行（多个 SDD change / feature 分支同时开）时遵循：

- **硬规则**：同 crate 的不同 worktree **禁止**共用 `CARGO_TARGET_DIR`。Cargo fingerprint 不区分包绝对路径，可错误标 `Fresh` 并跑到别的树编出的二进制（1.97.x 已复现）。
- 每树独立 target：
  ```bash
  eval "$(just cargo-wt-env)"        # 或 source scripts/cargo_worktree_env.sh
  ```
  输出 `CARGO_TARGET_DIR=~/.cache/cargo-targets/<repo>/<wt-key>/`，按仓库根路径哈希隔离；脚本见 `scripts/cargo_worktree_env.sh`（维护脚本，不进 qa）。
- **共享层**：`~/.cargo`（registry/git）+ sccache（`RUSTC_WRAPPER=sccache`）跨树安全；`SCCACHE_CACHE_SIZE` 防止缓存反噬磁盘。
- 清盘：删除旧 worktree 后顺手 `rm -rf ~/.cache/cargo-targets/<repo>/<对应key>/`；大 target 内部结构（debuginfo / incremental）处置思路见 skill `rust-build-tune`。

## 提交与测试

- Conventional Commits；开 PR 前 `just qa`。
- 单轨 feature-as-spec（llman ≥0.0.68）：每个 capability 恰一个 `llmanspec/specs/<cap>/<cap>.feature`（`@human` 规则 + `@executable` 验收场景）；流程见 `llmanspec/AGENTS.md`。**禁止** `solidify` / `change delta` / 新建 `*.feature.delta.toon`。
- 测试分层：BDD 管端到端编排；单测管纯数据/组件边界，不重复全链路。
- 实现计划变更后同步 `llmanspec/`；读代码优先 `rg`。

## llmanspec 命名

- capability = 领域名词、kebab-case；purpose / statement / scenario **中文**。
- 前缀按层：`package-tui-*` / `app-tui-*` / `agent-*` / `infra-*` / `protocol-*` / `cli-*` / `server-*` / `test-*` 等。细则：`llmanspec/AGENTS.md`。

## Specs 约束层级（产品级优先）

- specs 的 requirement 约束落在**产品级**：可观察行为、产品语义、数据契约、验证结果；**禁止硬约束代码组织**——具体路径、文件/模块名、类型名、行数、方法归属、迁移清单（除非是**大的组织方向**：分层依赖、端口 seam、crate 边界、组合根职责、跨面同源）。
- 代码组织演进（重构、改名、移动）不要求改 spec；spec 只随产品行为变化而变。代码是真值源，spec 不追平组织细节。
- 已删除对象（类型/模块/方法）的引用条款随删除一并清理，不保留「防复活」清单（除非真实回归风险）。
- 细则：`llmanspec/AGENTS.md` → 「spec 约束层级」；组织细节维护落在 `src/AGENTS.md`，不落在 specs。

## Skills

SDD：`llman-sdd-*`（含 `llman-sdd-quick` 快速路径）。应用面：`l8ng-write-surface`、`l8ng-audit-dead-code`、`l8ng-write-tui`。TUI 验证：`test-tui-harness`。观测窄读：`xylitol-inspect-runtime-logs`。构建/磁盘：`rust-build-tune`。TUI 参考：`tui-expert-of-codex`、`l8ng-terminal-tui-differential-rendering`（`l8ng-*` 均在 `~/.config/llman/skills/` 维护，不在本仓库，避免漂移）。外部锁定：`langfuse`（经 `skills-lock.json` 管理）。

## 编写与维护 AGENTS.md

`AGENTS.md` = **稳定操作边界**，不是 README / 进度板。

| 写 | 不写 |
|---|---|
| WHAT、硬约束、依赖方向、开闭规则 | 进度、行数快照、commit 列表、待办板 |
| HOW 指针（skill / 命令 / `docs/architecture`） | 长教程、完整测试矩阵、易腐文件地图 |
| 与父级不同的本地例外 | 重复父级长文；钉死易变文件路径 / change id |

**偏好**：全局规则与架构不变量；落点用层名（`agent`/`infra`/…），细节去读代码。子目录只补本面例外。

放哪：全仓规则 → 根；目录边界 → 该目录 `AGENTS.md`；多步 how-to → skill；临时笔记 → `_HANDOFF` / `*.tmp.md`（勿升格规范）；**change 作用域调研** → `llmanspec/changes/<id>/research/`（细则 `llmanspec/AGENTS.md`，勿堆 `docs/research/`）。

维护：六个月后是否仍真？否则不要进 AGENTS。复杂度闸与拆分默认策略在 `src/AGENTS.md`；**不**另维护超标表。
