---
name: xylitol-dev
description: >-
  xylitol 仓库的团队开发规范与工程方法总入口（不是产品 / 架构正文）。在本仓「按规范做事」先读本 skill，
  再按路由表读对应 reference。即使只说「这个改动该放哪」「日志怎么看」也应触发。

  以下相关内容也要触发（脚本生成，勿手改）：写或改 AGENTS.md（抗衰老 / 墓碑描述清理 / 项目 skills 同审 / 文档放哪层 / 写法正反例）；仓库瘦身盘点（examples / scripts / 子包调用方交叉、删留闸、一次性 lab 进 freeze research）；提交与 PR 卫生（Conventional Commits / archive(sdd) 收口 / 会话交接 handoff）；依赖选型与已退役件（jsonrpsee / axum / tiktoken / RemoteCount / 线协议 fory / token 计数 Heuristic）；docs 落点与产品↔实现闭环（GitHub FR/roadmap → SDD → architecture / 术语表 / 黑话对照 / 内部编号语义尾 / 跨端同源）；日志与 provider-trace 窄读排障（obs-* 子命令 / spinner 卡住 / 滚轮延迟 / RUST_LOG / XYLITOL_DEBUG / Langfuse / OTLP）；门禁 just qa / just qa-e2e（scripts/check_*.py 接线 / prek 与 qa 正交 / 复杂度门禁 cccc / 元门禁）；测试分层与 lab_ 试验命名（快照 insta / wire 契约 wiremock / nextest 并行与 #[serial] / 禁 thread::sleep）；工具链与依赖 bump（rustup / dated nightly / MSRV rust-version / cargo update Cargo.lock / worktree CARGO_TARGET_DIR）；TUI 分层验证 harness（按键→状态 / snapshot INSTA_UPDATE / 真终端 E2E tmux+PTY / agent_demo）
---

# xylitol 团队开发规范（总入口 / 路由）

**边界**：本 skill 是团队**开发方法论**的单一内聚落点——门禁、测试分层、文档落点、提交协作、依赖与工具链、验证 harness、观测排障。代码架构不变量真源在 `src/AGENTS.md`，产品行为合约在 `llmanspec/specs/`，产品心智在 `docs/architecture/`；本 skill **不复述**它们，只讲**怎么做**与**落点**。

## 先读顺序

1. 本文件（路由 + 硬原则）。
2. 按任务读下表对应 reference。
3. 要**产品 / 架构真值**时才去 `docs/` / `src/AGENTS.md` / `llmanspec/specs/`。

## 路由表

| 任务 / 关键词 | 读 |
|---|---|
| 写 / 改 `AGENTS.md`、抗衰老纪律、墓碑描述清理 | `references/agents-authoring.md` |
| 跑门禁 `just qa` / `qa-e2e`、加 `scripts/check_*.py`、prek 与 qa 正交、复杂度门禁 | `references/qa-gates.md` |
| 该写什么测试、测试分层、`lab_` 命名、nextest 并行纪律、禁 `thread::sleep` | `references/test-layering.md` |
| 产品↔实现闭环、GitHub FR 兑现后迁 architecture、术语表用法 | `references/docs-loop.md` |
| 提交信息、PR、会话交接 handoff | `references/commit-and-collab.md` |
| 依赖选型、已退役件、何时再引入（HTTP 框架 / JSON 通道 / OpenAPI / 本地词表） | `references/deps-candidates.md` |
| Rust nightly pin、`rust-version`、`cargo update`、Cargo.lock 漂移、MSRV | `references/toolchain-bump.md` |
| TUI 分层验证（按键→状态 / snapshot / 时序 / 真终端 E2E） | `references/tui-harness.md` |
| 日志 / provider-trace 窄读、通道混写、spinner 冻结排障 | `references/observability.md` |
| 仓库瘦身（examples / scripts / 子包叶子、删留闸） | `references/cleanup-triage.md` |

## 硬原则（in-scope MUST）

- **行为合约变更走 SDD**（explore → propose → apply → verify → archive）：真源 `llmanspec/AGENTS.md`。纯代码组织演进导致 spec 过期（主语 / 路径 / 迁移条款）→ **直接编辑** live `.feature` 并直接 commit，仍 MUST 过 `llman-sdd validate`。
- **全量门禁 `just qa`**；需要真终端（PTY / tmux）再加 `just qa-e2e`。门禁组成以 `justfile` 为 SSOT，勿在本 skill 内联枚举；非变更门禁脚本 `scripts/check_*.py` MUST 经 wiring 进 `qa`。
- **单一真值**：规则（稳定边界）进 `AGENTS.md`；how-to 进本 skill；产品语义进 `docs/`；行为合约进 `llmanspec/`。同一事实只在一处写全，他处用指针。
- **提交** Conventional Commits。
- **试验**统一 `lab_` 前缀；禁止再引入 `evidence_` / `experiment_`。

## 家族与命名

- 本 skill 是团队开发方法论的**唯一**落点。新增团队 how-to 先进对应 reference；某 reference 撑大或触发面清晰独立时，才考虑提升为独立 skill。
- **skill 之间不做横向引用**（跨 skill 指针易漂移）：需要的信息在本 skill 内用自己的 reference 承载；仓内文档（`src/AGENTS.md` / `docs/` / `llmanspec/`）可以指。
- 独立任务型 skill 的名字本身就是触发信号，不强套 `xylitol-dev-` 前缀。
