<!-- LLMANSPEC:START -->

# LLMAN 规范驱动开发

本项目使用 llman SDD。阅读 `llmanspec/config.yaml` 了解 SDD 命令行为配置，以及 `llmanspec/AGENTS.md` 获取项目附加规则。

## SDD 流水线

使用 `/llman-sdd-explore` 开始，然后按照 pipeline：`/llman-sdd-propose` → `/llman-sdd-apply` → `/llman-sdd-verify` → `/llman-sdd-archive`。

保留此托管块，便于 `llman-sdd init --update` 刷新。
<!-- LLMANSPEC:END -->

# 仓库级 Agent 指南

用与用户相同的语言回复。`xylitol` 是 **开箱即用的个人 coding agent**（LLM 增强开发工具包）。Rust workspace；目标 crate 图与依赖方向见 `src/AGENTS.md`。现行成员以根 `Cargo.toml` 为准。开发由 llman SDD 驱动。

术语表（黑话 ↔ 标准词 ↔ 英文对照总表；散文写作遇黑话首现时对照）：
@docs/architecture/术语表.md

## 产品定调

个人**开箱即用**的 coding harness：不是扩展市场 / 插件平台，不得为“未来插件”堆抽象；`xylitol-tui` 为源自 pi-tui 的独立 fork，不追平上游。取舍细则：`docs/architecture/产品分层总览.md`；扩展分界：`docs/architecture/扩展与开闭.md`。

- **Trust / MCP**：Trust 门禁**项目本地资源是否加载**（工具开箱 allow-all）；MCP 配置驱动、未配置零装配。产品规则：`docs/architecture/信任与项目门禁.md` / `扩展能力-MCP.md`；实现边界：`src/AGENTS.md`。
- **产品端**：Print + TUI（TTY 默认）已开放；产品 TUI 默认 attach 本机 Host，未在听失败；Print 与库嵌入可同进程。端规则：`src/app/tui/AGENTS.md`；拓扑与线协议：`docs/architecture/库与多客户端.md` / `远程体验与线协议.md`。
- **CS 角色 / 产品信封**：端是 client（键、画、TTY 等端侧能力），模型 / 会话 / MCP / trust 是 host 操作器角色；跨进程交互走四象限 RPC 信封（Command/Event 是载荷）。硬边界：`src/AGENTS.md`；产品叙事：`docs/architecture/库与多客户端.md`。
- **跨端公共体验**（已开放端 Print / TUI / Host）：动作语义一套学习成本；细则 `docs/architecture/库与多客户端.md`。未开放端（gpui 等）见 GitHub feature request，不得写成现行必须。
- 产品架构总览：`docs/architecture/`；未兑现方向：GitHub Issues（roadmap [#28](https://github.com/StrayDragon/xylitol/issues/28)，FR 各一个）；文档写法见 `docs/AGENTS.md`。
- TUI 信息呈现固定词（滚动提示、通知条、待办栏、尾插…）：`docs/architecture/TUI信息呈现与固定区词汇.md`（端约束见 `src/app/tui/AGENTS.md`）。新文只用表内词；弃用 / 旧称统一收在该表“弃用”对照行，正文不复述。
- 产品 TUI 视觉色板：`src/app/tui/DESIGN.md`（frontmatter SSOT；`just qa-check-tui-tokens` 对照 Palette）。

## 工作原则

| 做 | 不做 |
|---|---|
| 目标不清先确认范围；改动聚焦 | 边界未清就动手；顺手改无关代码 |
| 黑话首次出现时附标准词或英文（总表：顶部 @ 引入的术语表）；内部编号（`c####` / `ath##`）单独引用时带 ≤12 字语义尾 | 黑话不加对照直接用；编号不带语义尾造成检索死端 |
| 代码是真值源；架构规则读 `src/AGENTS.md`；设计史查 `llmanspec/changes/archive/` | 凭记忆改架构；向 archive 要现行规则 |
| 动代码前读相关代码，沿目录树遵循最近的 `AGENTS.md` | 跳过上下文直接改 |

## 目标 crate（摘要）

Host 侧 crate 边界按替换、开关、生命周期划，**不得**按 `app / agent / infra` 各拆一 crate。依赖与 seam 纪律 **只**在 `src/AGENTS.md`。此处一行角色：

| crate | 角色 |
|---|---|
| `xylitol-plugin-sdk` | 词表 + 端口 + 错误（插件作者只依赖这一包） |
| `xylitol-protocol` | 四象限信封 + fory-v3。不认识端口，不认识 `XyEvent` |
| `xylitol-runtime` | 执行权威：`XyDriver` + trust + 会话槽 + ReAct 内环 + 唤醒准入。不接线、不依赖插件、不依赖信封 |
| `xylitol-serve` | 载体：监听 + `XyEvent` ↔ v3 mapping |
| `xylitol-official-plugin-*` | 官方 Host 插件（七个：session、compaction、provider-openai、provider-anthropic、tools、mcp、hooks）。不是市场，不是 dylib |
| `xylitol-app-tui` / `xylitol-app-print` | 产品端。TUI 只认信封；Print 同进程直调 runtime，不编 fory |
| `xylitol-tui` | 通用 TUI 引擎（零引用产品） |
| 根包 `xylitol` | 组合根：argv + 默认接线。唯一列出官方插件的 `Cargo.toml` |

`plugin-sdk` ∥ `protocol` ∥ `xylitol-tui`。runtime → sdk。插件 → sdk。serve → runtime + protocol。app-tui → protocol + tui。根包 → 插件 + runtime + 端 + 载体。插件之间不得 `use`。

开箱 vs 配置后置：产品 TUI 默认 attach 本机监听（`server` feature 可关，default 仍开）。MCP 有配置才装配（`mcp` feature 可关，default 仍开）。`otel` feature default 开，配置默认 none。

## 编码规则

| 必须 | 不得 |
|---|---|
| 遵循 `rustfmt.toml` / `.editorconfig`；排版交给 `just fmt` / `just lint` | 在 AGENTS 里重复排版细则 |
| 命名：snake_case 模块与函数，PascalCase 类型 | 自创命名风格 |
| 模块边界贴合目标 crate 与 crate 内分层；能内联不套 wrapper | 为对称性加 wrapper；加无要求的兼容 shim |
| 优先穷举类型 / 注册表；公共形状让 rust-analyzer 能跳转 | 字符串 magics；跳转不到的公共形状 |
| `Xy*` 库契约与外部库包装准则：`src/AGENTS.md`“三层契约与 `Xy*`” | 凭感觉决定包不包一层 |

## Provider 交付范围

交付 **OpenAI 兼容** 与 **Anthropic Messages** 两族（OAuth 等纳入是显式决策，不默认承诺）；网关 courtesy（如 Zen 会话归因）允许。产品细则：`docs/architecture/多厂商模型.md`；开闭规则（业务只依赖 `XyModel`，新兼容端 = 方言插件 / 配置）：`src/AGENTS.md`“Provider 与消息”。

## 命令与验证

- 日常：`just setup` / `fmt` / `lint` / `test` / `test-tui`；全量门禁 `just qa`（或 `ci`）；需要真终端（PTY / tmux）再加 `just qa-e2e`。新机器先 `just setup`；缺工具在第一次触发 recipe 时安装或失败。
- 非变更门禁脚本 `scripts/check_*.py` **必须**经 wiring 进 `qa`；维护脚本不进门禁。
- **prek 与 `just qa` 正交**：prek hook 只承担文本 / 元数据卫生与 commit-msg 规约；cargo 级门禁（fmt / clippy / test / doc-test）**不得**拆成独立 hook 进 prek；push 一致性由 CI 兜底。
- 探查：`cargo run -- --help`；文档构建与检查：`just doc` / `just qa-doc-check`。
- 一次性探针只落 `scripts/lab_*.py`（或包 examples `lab_*`）并直接 python 调用，**不**新增 just recipe；重复使用或需流程化才升 skill / 门禁。引擎 demo 入口在 `crates/xylitol-tui/justfile`。
- 门禁组成（quiet/verbose、真实网关 lab、复杂度、元门禁）细则：`xylitol-dev` skill `references/qa-gates.md`。

### 试验 / 真实网关验证命名（`lab_`）

试验性证据与真实网关探针 **统一前缀 `lab_`**；新试验 **必须**用 `lab_`，**不得**再引入 `evidence_` / `experiment_` 前缀。四形态 × 是否进 `just qa` 的矩阵与跑法：`xylitol-dev` skill `references/test-layering.md`。

## Worktree 并行开发

- **硬规则**：同 crate 的不同 worktree **不得**共用 `CARGO_TARGET_DIR`（Cargo fingerprint 不区分包绝对路径，会错误复用别的树编出的产物）。
- 每个工作区先 `eval "$(just cargo-wt-env)"`（或 `source scripts/cargo_worktree_env.sh`）；路径布局见该脚本 `--help`。磁盘清盘 / 堆积考古无固化手册——具体行为随工具链版本变化（历史调优结论会过期），遇到时按当次 rust 版本即时调研。
- Linux 开发链接走 mold；`-Zshare-generics` 与 mold 写在同一份 `.cargo/config.toml`（`[target]` rustflags 会覆盖 `[build]`，不得拆开）。

## 提交与测试

- Conventional Commits；开 PR 前 `just qa`。提交与协作细则：`xylitol-dev` skill `references/commit-and-collab.md`。
- 单轨 feature-as-spec、capability 命名、spec 约束层级（产品级 WHAT，不得钉代码组织）：`llmanspec/AGENTS.md`。**不得**使用 `solidify` / `change delta` / 新建 `*.feature.delta.toon`。
- 测试分层：BDD 管端到端编排；单测管纯数据 / 组件边界，不重复 BDD 场景。细则：`xylitol-dev` skill `references/test-layering.md`。
- 实现计划变更后写入 `llmanspec/`；读代码优先 `rg`。

## Skills

团队开发规范（门禁 / 测试分层 / 文档落点 / 提交协作 / 依赖候选 / 工具链 bump / TUI 验证 / 观测排障）总入口：`xylitol-dev`（正文在 `references/`）。SDD：`llman-sdd-*`（含 `llman-sdd-quick` 快速路径）。应用端：`l8ng-write-surface`、`l8ng-audit-dead-code`、`l8ng-write-tui`。TUI 参考：`l8ng-terminal-tui-differential-rendering`（`l8ng-*` 均在 `~/.config/llman/skills/` 维护，不在本仓库，避免漂移）。外部锁定：`langfuse`（经 `skills-lock.json` 管理）。构建 / 磁盘调优：按当次 rust 版本即时调研，不固化结论。

## 编写与维护 AGENTS.md

`AGENTS.md` = **稳定操作边界**，不是 README / 进度板。只收**硬约束 / 稳定原则 / 宽泛索引（HOW 指针）**；易腐细节以代码 / skill / spec 为 SSOT，此处至多留指针。**移除即改写**：不留“曾／已退役／原名”墓碑。**同审** `.agents/skills/` 项目 skills：事实性断言对照代码复核，漂移即修。

落点：全仓规则 → 根；目录边界 → 该目录 `AGENTS.md`；多步 how-to → skill（团队 how-to 进 `xylitol-dev`）；临时笔记 → `_HANDOFF` / `*.tmp.md`；**change 作用域调研** → `llmanspec/changes/<id>/research/`。写什么 / 不写什么与正反例速查表：`xylitol-dev` skill `references/agents-authoring.md`。
