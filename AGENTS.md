<!-- LLMANSPEC:START -->
# LLMAN 规范驱动开发

本项目使用 llman SDD。阅读 `llmanspec/config.yaml` 了解 SDD 命令行为配置，以及 `llmanspec/AGENTS.md` 获取项目附加规则。

## SDD 流水线

使用 `/llman-sdd-explore` 开始，然后按照 pipeline：`/llman-sdd-propose` → `/llman-sdd-apply` → `/llman-sdd-verify` → `/llman-sdd-archive`。

保留此托管块，便于 `llman sdd init --update` 刷新。
<!-- LLMANSPEC:END -->

# 仓库级 Agent 指南

用与用户相同的语言回复。`xylitol` 是 **开箱即用的个人 coding agent**（LLM 增强开发工具包）：Rust workspace——主 crate 加 `packages/`（`xylitol-tui` TUI 引擎、`xylitol-ai-bridge` provider 方言桥）；开发由 llman SDD 驱动。

术语表（黑话 ↔ 标准词 ↔ 英文对照总表；散文写作遇黑话首现时对照）：
@docs/architecture/术语表.md

## 产品定调

个人**开箱即用**的 coding harness：不是扩展市场 / 插件平台，不为「未来插件」堆抽象；`xylitol-tui` 为源自 pi-tui 的独立 fork，不追平上游。取舍细则：`docs/architecture/产品分层总览.md`；扩展分界：`docs/architecture/扩展与开闭.md`。

- **Trust / MCP**：Trust 门禁**项目本地资源是否加载**（工具开箱 allow-all）；MCP 配置驱动、未配置零装配。产品规则：`docs/architecture/信任与项目门禁.md` / `扩展能力-MCP.md`；实现边界：`src/AGENTS.md`。
- **产品端**：Print + TUI（TTY 默认）已开放；产品 TUI 默认 attach 本机 Host，未在听失败；Print 与库嵌入可同进程。端规则：`src/app/tui/AGENTS.md`；拓扑与线协议：`docs/architecture/库与多客户端.md` / `远程体验与线协议.md`。
- **CS 角色 / 产品信封**：端是 client（键、画、TTY 等端侧能力），模型 / 会话 / MCP / trust 是 host 操作器角色；跨进程交互走四象限 RPC 信封（Command/Event 是载荷）。硬边界：`src/AGENTS.md`；产品叙事：`docs/architecture/库与多客户端.md`。
- **跨端公共体验**（TUI ↔ gpui 桌面）：动作语义一套学习成本；唯一约束板 `docs/roadmaps/跨端同源.md`。
- 产品架构总览：`docs/architecture/`；候选方向：`docs/roadmaps/`（不维护进度列）；文档闭环：`docs/AGENTS.md`。
- TUI 信息呈现固定词（滚动提示、通知条、尾插…）：`docs/architecture/TUI信息呈现与固定区词汇.md`（端约束见 `src/app/tui/AGENTS.md`）。
- 交互设计稿：仓库顶层 `designing/`（`just open-designing`）；运行时真值、tui-lab SOP 与快捷键约定见 `designing/AGENTS.md`。

## 工作原则

| 做 | 不做 |
|---|---|
| 目标不清先对齐；改动聚焦 | 边界未清就动手；顺手改无关代码 |
| 黑话首次出现时附标准词或英文（总表：顶部 @ 引入的术语表）；内部编号（`c####` / `ath##`）单独引用时带 ≤12 字语义尾 | 黑话不加对照直接用；编号不带语义尾造成检索死端 |
| 代码是真值源；架构规则读 `src/AGENTS.md`；设计史查 `llmanspec/changes/archive/` | 凭记忆改架构；向 archive 要现行规则 |
| 动代码前读相关代码，沿目录树遵循最近的 `AGENTS.md` | 跳过上下文直接改 |
| 提交不加 co-author / 不暴露 agent 身份 | 提交信息带 AI 痕迹 |

## 分层地图（摘要）

主 crate 内逻辑分层（**不**拆 crate）。依赖与 seam 纪律 **只**在 `src/AGENTS.md`；此处一行角色：

| 层 | 角色 |
|---|---|
| `protocol/` | wire + ports + 根上共享类型 |
| `agent/` | 薄编排（ReAct、capabilities、投影 projection） |
| `infra/` | ports 实现与 vendor |
| `app/` | 应用端 + `core` seam |
| `utils/` | 纯叶工具（仅 std；非 `Xy*` 稳定 API） |
| `packages/xylitol-tui` | 通用 TUI 引擎（零引用主 crate） |

`app → agent|infra → protocol`；`agent` ↛ `infra`；`infra` ↛ `agent`。

## 编码规则

| 要 | 不要 |
|---|---|
| 遵循 `rustfmt.toml` / `.editorconfig`；排版交给 `just fmt` / `just lint` | 在 AGENTS 里重复排版细则 |
| 命名：snake_case 模块与函数，PascalCase 类型 | 自创命名风格 |
| 模块边界贴合分层；能内联不套 wrapper | 为对称性加 wrapper；加无要求的兼容 shim |
| 优先穷举类型 / 注册表；公共形状让 rust-analyzer 能跳转 | 字符串 magics；跳转不到的公共形状 |
| `Xy*` 库契约与外部库包装准则：`src/AGENTS.md`「三层契约与 `Xy*`」 | 凭感觉决定包不包一层 |

## Provider 交付范围

交付 **OpenAI 兼容** 与 **Anthropic Messages** 两族（OAuth 等纳入是显式决策，不默认承诺）；网关 courtesy（如 Zen 会话归因）允许。产品细则：`docs/architecture/多厂商模型.md`；开闭规则（业务只依赖 `XyModel`，新兼容端 = adapter/配置）：`src/AGENTS.md`「Provider 与消息」。

## 命令与验证

- 日常：`just setup` / `fmt` / `lint` / `test` / `test-tui`；全量门禁 `just qa`（或 `ci`）；需要真终端（PTY / tmux）验证时再 `just qa-e2e`。
- `just qa` 串行跑 `test-live-provider`（连接真实网关的 lab；配置路径、`enabled` 门禁与 skip 语义见 `justfile` 与 `just gen-live-provider-example`）。产品示例配置 `configs/example.yaml` 由 `just gen-config-example` 生成（改 `scripts/gen_config_example.py` 模板，勿直接手改产物）。
- `qa` 输出默认 quiet；`just qa normal` / `verbose` 或 `JUST_VERBOSITY=` 调整。
- 非变更门禁脚本 `scripts/check_*.py` **MUST** 经 wiring 进 `qa`；维护脚本不进门禁。
- 探查：`cargo run -- --help`；文档构建与检查：`just doc` / `just doc-check`。

### 试验 / 真实网关验证命名（`lab_`）

试验性证据与真实网关探针 **统一前缀 `lab_`**（不再用 `evidence_` / `experiment_` / 试验性 `live_` 二进制名）：

| 形态 | 约定 | 是否进 `just qa` |
|---|---|---|
| `packages/*/examples/lab_*.rs` | 人跑维护 lab（`cargo run -p … --example lab_…`） | **否** |
| `packages/*/tests/lab_*.rs` | 可连接真实网关的 lab 二进制；是否纳入门禁看 just 接线 | **仅**已接线者（见 `just test-live-provider`） |
| `#[test] fn lab_*`（无 `#[ignore]`） | 门禁内契约实验（无网或假网） | **是**（随 crate 测） |
| `#[ignore]` 的 `lab_*` 测试 / `src/**/lab_*.rs` lab 模块 | 人跑维护 lab（模块 doc 给命令） | **否** |

配置文件名 `live-provider.yaml` / recipe `test-live-provider` 保留「live 网关」语义，与代码符号前缀无关。新试验 **MUST** 用 `lab_`；禁止再引入 `evidence_` / `experiment_` 前缀。

## Worktree 并行开发

- **硬规则**：同 crate 的不同 worktree **禁止**共用 `CARGO_TARGET_DIR`（Cargo fingerprint 不区分包绝对路径，会错误复用别的树编出的产物）。
- 每个工作区先 `eval "$(just cargo-wt-env)"`（或 `source scripts/cargo_worktree_env.sh`）；路径布局、sccache 共享与清盘细则见 skill `rust-build-tune`（`references/disk-and-worktree.md`）。

## 提交与测试

- Conventional Commits；开 PR 前 `just qa`。
- 单轨 feature-as-spec、capability 命名、spec 约束层级（产品级 WHAT，禁止钉代码组织）：`llmanspec/AGENTS.md`。**禁止** `solidify` / `change delta` / 新建 `*.feature.delta.toon`。
- 测试分层：BDD 管端到端编排；单测管纯数据/组件边界，不重复全链路。
- 实现计划变更后同步 `llmanspec/`；读代码优先 `rg`。

## Skills

SDD：`llman-sdd-*`（含 `llman-sdd-quick` 快速路径）。应用端：`l8ng-write-surface`、`l8ng-audit-dead-code`、`l8ng-write-tui`。TUI 验证：`test-tui-harness`。观测窄读：`xylitol-inspect-runtime-logs`。构建/磁盘：`rust-build-tune`。工具链与 crates.io 依赖 bump：`xylitol-bump-toolchain`。TUI 参考：`tui-expert-of-codex`、`l8ng-terminal-tui-differential-rendering`（`l8ng-*` 均在 `~/.config/llman/skills/` 维护，不在本仓库，避免漂移）。外部锁定：`langfuse`（经 `skills-lock.json` 管理）。

## 编写与维护 AGENTS.md

`AGENTS.md` = **稳定操作边界**，不是 README / 进度板。

| 写 | 不写 |
|---|---|
| WHAT、硬约束、依赖方向、开闭规则 | 进度、行数快照、commit 列表、待办板 |
| HOW 指针（skill / 命令 / `docs/architecture`） | 长教程、完整测试矩阵、易腐文件地图 |
| 与父级不同的本地例外 | 重复父级长文；复述他处 SSOT 已写全的事实（用指针替代）；钉死易变文件路径 / change id |

**偏好**：全局规则与架构不变量；落点用层名（`agent`/`infra`/…），细节去读代码。子目录只补本端例外。

放哪：全仓规则 → 根；目录边界 → 该目录 `AGENTS.md`；多步 how-to → skill；临时笔记 → `_HANDOFF` / `*.tmp.md`（勿升格规范）；**change 作用域调研** → `llmanspec/changes/<id>/research/`（细则 `llmanspec/AGENTS.md`，勿堆 `docs/research/`）。

维护：六个月后是否仍真？否则不要进 AGENTS。复杂度门禁与拆分默认策略在 `src/AGENTS.md`；**不**另维护超标表。
