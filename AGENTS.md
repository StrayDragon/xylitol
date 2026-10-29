<!-- LLMANSPEC:START -->

# LLMAN 规范驱动开发

本项目使用 llman SDD。阅读 `llmanspec/config.yaml` 了解 SDD 命令行为配置，以及 `llmanspec/AGENTS.md` 获取项目附加规则。

## SDD 流水线

使用 `/llman-sdd-explore` 开始，然后按照 pipeline：`/llman-sdd-propose` → `/llman-sdd-apply` → `/llman-sdd-verify` → `/llman-sdd-archive`。

保留此托管块，便于 `llman-sdd init --update` 刷新。
<!-- LLMANSPEC:END -->

# 仓库级 Agent 指南

`xylitol` 是一个 **coding agent** 实现

Rust workspace；目标 crate 图与依赖方向见 `src/AGENTS.md`。现行成员以根 `Cargo.toml` 为准。开发由 llman SDD 驱动。

术语与写作：黑话首现时附标准词或英文；`规则:` 标题用标准词（细则 `llmanspec/AGENTS.md`「语言」）。内部编号（`c####` / `ath##`）单独引用时带 ≤12 字语义尾。

## 产品定调

个人**开箱即用**的 coding harness。插件生态（第三方可写可装）是方向、未交付；现行扩展入口 = 官方插件 + 配置 + MCP / shell hook。不得为未交付市场堆清单 / 注册中心 / dylib。`xylitol-tui` 为源自 pi-tui 的独立 fork，不追平上游。分层与开闭：`src/AGENTS.md`；可验证行为：`llmanspec/specs/`。

- **Trust / MCP**：Trust 门禁**项目本地资源是否加载**（工具开箱 allow-all）；MCP 配置驱动、未配置零装配。产品规则：`llmanspec/specs/domain-security/`、`app-tui-trust/`、`infra-mcp/`；实现边界：`src/AGENTS.md`。
- **Loop 参与**：插件经 rutis 生命周期注册 loop 中间件（七阶段、控制枚举）**参与** ReAct 决策；core 持 turn 边界 / 取消 / 权限终裁，插件不得整段替换 loop。产品规则：`llmanspec/specs/agent-runtime/`；实现边界：`src/AGENTS.md`「Loop 中间件」。
- **产品端**：Print + TUI（TTY 默认）已开放；产品 TUI 默认 attach 本机 Host，未在听失败；Print 与库嵌入可同进程。端规则：`crates/xylitol-app-tui/src/AGENTS.md`；拓扑与线协议：`llmanspec/specs/protocol-app/`、`server-core/`。
- **CS 角色 / 产品信封**：端是 client（键、画、TTY 等端侧能力），模型 / 会话 / MCP / trust 是 host 操作器角色；跨进程交互走四象限 RPC 信封（JSON-RPC 2.0 文本：`POST /rpc` unary + `WS /rpc` 双向 text 帧；Command/Event 是载荷）。硬边界：`src/AGENTS.md`。
- **跨端公共体验**（已开放端 Print / TUI / Host）：动作语义一套学习成本；细则 `llmanspec/specs/protocol-app/` 与 `user-experience/`。未开放端（gpui 等）见 GitHub feature request，不得写成现行必须。
- 未兑现方向：GitHub Issues（roadmap [#28](https://github.com/StrayDragon/xylitol/issues/28)，FR 各一个；第三方插件 [#29](https://github.com/StrayDragon/xylitol/issues/29)）。文档落点：`xylitol-dev` skill `references/docs-loop.md`。
- TUI 信息呈现固定词（滚动提示、通知条、待办栏、尾插…）：`llmanspec/specs/app-tui-fixed-zone/`（端约束见 `crates/xylitol-app-tui/src/AGENTS.md`）。
- 产品 TUI 视觉色板：`crates/xylitol-app-tui/src/DESIGN.md`（frontmatter SSOT；`just qa-check-tui-tokens` 对照 Palette）。

## 工作原则

| 做 | 不做 |
|---|---|
| 目标不清先确认范围；改动聚焦 | 边界未清就动手；顺手改无关代码 |
| 黑话首次出现时附标准词或英文（见顶部「术语与写作」）；内部编号（`c####` / `ath##`）单独引用时带 ≤12 字语义尾 | 黑话不加对照直接用；编号不带语义尾造成检索死端 |
| 代码是真值源；架构规则读 `src/AGENTS.md`；设计史查 `llmanspec/changes/archive/` | 凭记忆改架构；向 archive 要现行规则 |
| 动代码前读相关代码，沿目录树遵循最近的 `AGENTS.md` | 跳过上下文直接改 |

## 目标 crate（摘要）

Host 侧 crate 边界按替换、开关、生命周期划，**不得**按 `app / agent / infra` 各拆一 crate。依赖与 seam 纪律 **只**在 `src/AGENTS.md`。此处一行角色：

| crate | 角色 |
|---|---|
| `xylitol-plugin-sdk` | 词表 + 端口 + 错误 + `rt` 再导出（插件作者只依赖这一包） |
| `xylitol-protocol` | 四象限信封（JSON-RPC 2.0 文本）+ `PROTOCOL_VERSION`。不认识端口，不认识 `XyEvent` |
| `xylitol-runtime` | 执行权威：`XyDriver` + trust + 会话槽 + ReAct 内环 + 唤醒准入。不依赖信封 |
| `xylitol-kernel` | 进程内核：`Kernel::boot`、核心插件、Builtins、admit。`rutis-loader` 只在这里 |
| `xylitol-serve` | 载体：监听 + JSON-RPC 分派；Host profile 插件 |
| `xylitol-wire` | `XyEvent` ↔ wire `Event` 的唯一映射（依赖 protocol + sdk） |
| `xylitol-official-plugin-*` | 官方可替换插件（session / compaction / provider-* / tools / todo / workflow / plan / mcp / hooks / config / obs）。不是市场，不是 dylib |
| `xylitol-app-tui` / `xylitol-app-print` | 产品端。TUI 只认线协议（经 `xylitol-wire` 映射）；Print 同进程直调，不编线协议 |
| `xylitol-tui` | 通用 TUI 引擎（零引用产品） |
| 根包 `xylitol` | argv + profile 选择。默认接线经 kernel |

`plugin-sdk` ∥ `protocol` ∥ `xylitol-tui`。`wire` → protocol + sdk。runtime → sdk。插件 → sdk。kernel → runtime + 插件 + sdk + loader + protocol + serve + app-print。serve → runtime + protocol + wire。app-tui → protocol + wire + tui。根包 → kernel + 端。插件之间不得 `use`（只经服务 / 事件）。

开箱 vs 配置后置：产品 TUI 默认 attach 本机监听（`server` feature 可关，default 仍开）。MCP 有配置才装配（`mcp` feature 可关，default 仍开）。`otel` feature default 开，配置默认 none。fff 检索工具源 default 开（`fff` feature 可关）；模型可见工具集由配置 `tools` 列表选择，替换工具默认优先、内置 `grep` / `find` 回落。

## 编码规则

| 必须 | 不得 |
|---|---|
| 遵循 `rustfmt.toml` / `.editorconfig`；排版交给 `just fmt` / `just lint` | 在 AGENTS 里重复排版细则 |
| 命名：snake_case 模块与函数，PascalCase 类型 | 自创命名风格 |
| 模块边界贴合目标 crate 与 crate 内分层；能内联不套 wrapper | 为对称性加 wrapper；加无要求的兼容 shim |
| 优先穷举类型 / 注册表；公共形状让 rust-analyzer 能跳转 | 字符串 magics；跳转不到的公共形状 |
| 线协议标识符按语义命名（方法名、事件 `type`、类型名）；协议版本只经 `PROTOCOL_VERSION` 与 `host.describe` 协商 | 标识符带版本段（`new_v3`、`V4`、`*_v3` 等）；为旧名保留别名或双解析路径（由 `just qa` 的版本命名闸卡住） |
| `Xy*` 库契约与外部库包装准则：`src/AGENTS.md`“三层契约与 `Xy*`” | 凭感觉决定包不包一层 |

## Provider 交付范围

交付 **OpenAI 兼容** 与 **Anthropic Messages** 两族（OAuth 等纳入是显式决策，不默认承诺）；网关 courtesy（如 Zen 会话归因）允许。产品细则：`llmanspec/specs/infra-provider/` 与插件侧 `provider-openai*`；开闭规则（业务只依赖 `XyModel`，新兼容端 = 方言插件 / 配置）：`src/AGENTS.md`“Provider 与消息”。

## 命令与验证

- 日常：`just setup` / `fmt` / `lint` / `test` / `test-tui`；全量门禁 `just qa`（或 `ci`）；需要真终端（PTY / tmux）再加 `just qa-e2e`。新机器先 `just setup`；缺工具在第一次触发 recipe 时安装或失败。
- 非变更门禁脚本 `scripts/check_*.py` **必须**经 wiring 进 `qa`；维护脚本不进门禁。
- **prek 与 `just qa` 正交**：prek hook 只承担文本 / 元数据卫生与 commit-msg 规约；cargo 级门禁（fmt / clippy / test / doc-test）**不得**拆成独立 hook 进 prek；push 一致性由 CI 兜底。
- 探查：`cargo run -- --help`；文档构建与检查：`just doc` / `just qa-doc-check`。
- 一次性探针只落 `scripts/lab_*.py`（或包 examples `lab_*`）并直接 python 调用，**不**新增 just recipe；重复使用或需流程化才升 skill / 门禁。引擎 demo 入口在 `crates/xylitol-tui/justfile`。
- 门禁组成（quiet/verbose、真实网关 lab、复杂度、元门禁、注释只读闸、版本命名闸）细则：`xylitol-dev` skill `references/qa-gates.md`。
- **Python 解释器**：门禁脚本需 `>=3.11`（`tomllib`）。`justfile` 与 BDD step 均自动挑选第一个可 `import tomllib` 的解释器（`python3.13` → `python3.12` → `python3.11` → `python3`），无需手工切 PATH；系统 3.9 会让门禁直接报 `No module named 'tomllib'` 而非静默跳过。`PyYAML` 已从门禁脚本移除（frontmatter / 示例配置改自解析），仅可选自检时使用。
- **注释闸（uncomment，只读）**：策略见根 `.uncomment.toml`；`just qa` 经 `scripts/check_uncomment.py` 仅跑 `uncomment --check --changed-lines`（不删注释、不 `--fix`）。默认移除行内说明与 `///`/`//!`/docstring（`remove_docs = true`）；保留 TODO/FIXME、工具指令与 `~keep`。追溯以 specs/提交信息为 SSOT，**改动行**上不得新增可移除注释。`uncomment` 须在 PATH（`uv tool install uncomment` 或 `pipx install uncomment`）；未安装时闸报错而非跳过。`XYLITOL_SKIP_UNCOMMENT=1` 仅为工具确实无法安装时的应急口，不得作为常规跑法。

### 试验 / 真实网关验证命名（`lab_`）

试验性证据与真实网关探针 **统一前缀 `lab_`**；新试验 **必须**用 `lab_`，**不得**再引入 `evidence_` / `experiment_` 前缀。四形态 × 是否进 `just qa` 的矩阵与跑法：`xylitol-dev` skill `references/test-layering.md`。

## Worktree 并行开发

- **硬规则**：同 crate 的不同 worktree **不得**共用 `CARGO_TARGET_DIR`（Cargo fingerprint 不区分包绝对路径，会错误复用别的树编出的产物）。
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
