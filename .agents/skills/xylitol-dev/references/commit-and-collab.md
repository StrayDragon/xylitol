---
brief: "提交与 PR 卫生（Conventional Commits / 无 co-author / 无 AI 痕迹 / archive(sdd) 收口 / 会话交接 handoff）"
---

# 提交与 AI 协作约定

## 提交卫生

- **Conventional Commits**；开 PR 前 `just qa`。
- **不加 co-author / 不暴露 agent 身份**（提交信息不得带 AI 痕迹）。
- 实现用 `feat` / `fix` / `refactor` / `perf` / `test` / `docs` / `build` / `ci` / `chore` / `style`；llman SDD 礼仪用 `chore(sdd):` / `docs(sdd):`；`change finalize` 收口用 `archive(sdd): <id>`；工具链升级用 `agentdev`（白名单见 `prek.toml`）。
- 单轨 feature-as-spec、capability 命名、spec 约束层级：`llmanspec/AGENTS.md`。**禁止** `solidify` / `change delta` / 新建 `*.feature.delta.toon`。

## 提交内容

- 实现计划变更后**同步** `llmanspec/`。
- SDD 闭环收尾优先 `llman-sdd change finalize`（单 `archive(sdd): <id>` 自动提交，减礼仪 commit）；详见 `llmanspec/AGENTS.md`。
- 产品 vs 流程分开提交：产品实现用 `feat`/`fix`/`refactor`；SDD 流程用 `chore(sdd):` / `docs(sdd):`。

## AI 协作约定

- **读代码优先 `rg`**，不凭记忆改。
- 动代码前读相关代码，沿目录树遵循**最近的** `AGENTS.md`；设法规避「顺手改无关代码」。
- 目标不清**先对齐**，边界未清不动手。
- **事故防复发**：agent 犯过的错应被机械化防复发（新增 `scripts/check_*.py`，docstring 记录案例）——见 `references/qa-gates.md`。
- 决策知识**不散失在聊天记录**：落进 `llmanspec/changes/*/design.md`、`PI_DELTAS` 台账、或本 skill 的 reference。

## 会话交接 handoff

- 临时笔记落 `_HANDOFF` / `*.tmp.md`，**勿**升格为规范。
- 交接给出：改动文件清单 + 门禁状态 + 未决点；不贴大段日志（观测窄读见 `references/observability.md`）。

## skill 与 AGENTS 同审

项目 skills（`.agents/skills/` 下 `xylitol-*` 等）与 AGENTS 同一**抗衰老纪律**：事实性断言（路径 / 命令 / 约束）对照代码复核，漂移即修。详见 `references/agents-authoring.md`。
