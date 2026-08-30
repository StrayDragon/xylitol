---
name: "llman-sdd-wayfinder"
description: "人类主动触发。把大型、一团乱的工作（超出单个 agent 会话容量）拆成一张决策地图，逐个解决决策直到路径清晰。仅手动触发，agent 禁止自动启用。"
metadata:
  version: "0.0.72"
---

# LLMAN SDD Wayfinder

一个又大又乱的工作来了——大到单个 agent 会话装不下，还裹着一团迷雾：从现在到**目的地**的路还看不见。这个 skill 不急着动手，而是先把路找出来。

它把路径画成 llman SDD 的 **change 依赖图**（`llman sdd graph`）：每个子工作（ticket）解决一个**决策**而非交付一块代码，逐个解决直到路径清晰。

## Pipeline 位置

辅助工具，用于主 pipeline 之前的**大型工作预规划**。地图清晰后，合并到主流程 `llman-sdd-propose`。

> 📍 这是独立可选 skill；地图清晰后 → `llman-sdd-propose`（把决策收拢为可建计划）。

## 核心原则

- **只规划，不动手**：每个 ticket 解决一个决策，地图完成于「路径清晰、无决策遗留」。想直接开干的冲动，通常是到了地图边界、该交接的信号。
- **用名字指代**：在所有给人看的叙述里，用 ticket 的标题名称指代，MUST NOT 用裸 id/编号。
- **单会话单 ticket**：每个会话只解决一个 ticket（查资料的 ticket 例外）。

## 地图结构

地图本身是一个 change（总纲 proposal），它的子决策是 `depends_on` 的子 change。用 `llman sdd graph <map-id> --scope active` 可视化当前**可着手项**。

地图的 `proposal.md` 结构：

```markdown
## Destination（目的地）
<走到地图终点是什么样——spec/决策/变更。一两行。>

## Notes（备注）
<领域；每会话应查的 skill；这次工作的常设偏好>

## Decisions so far（已定的决策）
<!-- 索引：每个已关闭 ticket 一行，结论要点 + 链接 -->

## Not yet specified（尚未清晰区）
<!-- 能预见但还说不清成 ticket 的；随推进逐渐变清晰 -->

## Out of scope（范围外）
<!-- 超出目的地的；关闭的 ticket，永不复活 -->
```

## Ticket 类型

每个 ticket 是一个子 change，带 `wayfinder:<type>` 标注（写在 proposal 的标题或 frontmatter）：

- **Research（查资料，agent 自跑）**：读文档/API/本地资源，把决策在等的事实查出来。委托 `llman-sdd-research` 后台解决。
- **Prototype（做原型，需人参与）**：用廉价粗糙的可运行物（throwaway 小程序或 UI 变体）把讨论具象化。
- **逐问深挖（需人参与）**：用 `llman-sdd-explore` 的逐问深挖分支一问一答走清。**默认类型**。
- **Task（杂活，需人或 agent 自跑）**：决策前必须先做的手动工作（注册服务、迁数据以看清形状）。

## 尚未清晰区

地图**故意**不完整。判断一个点该不该现在就立成 ticket，只看一条：**现在能不能把问题说清楚**（不是能不能回答）。
- 能说清楚 → 立 ticket（即使暂时被别的挡着）。
- 还说不清楚 → 写进 **尚未清晰区**（比 ticket 粗，一团模糊可能日后变成多个 ticket，也可能一个都不变）。

## 步骤

### 画地图
1. **命名目的地**：用 `llman-sdd-explore` 的逐问深挖分支钉死这趟地图要通往哪里。
2. **广度优先扫可着手项**：再次逐问深挖，扇开而非深挖一条，把开放决策和现在能迈的第一步浮出来。若**没有模糊点浮出**——路径已清晰、整个工作一个会话能装下——那就不需要地图，停下问用户想怎么做。
3. **创建地图**（总纲 change）：`llman sdd change new <map-id>`，填 Destination/Notes，Decisions-so-far 留空，模糊点写进尚未清晰区。
4. **创建现在能说清的 ticket**为子 change，然后用 `llman sdd graph` 接依赖边（第二步：先有 id 才能互引）。
5. 为每个查资料 ticket 启动 `llman-sdd-research` 后台 subagent。
6. 停——画图是单个会话的活，不要顺手解决任何决策。

### 推进地图
1. 加载地图（低分辨率视图，不用读每个 ticket 全文）。
2. 选 ticket（用户指定或取可着手项的第一个），先完成 Branch binding（`change start` 或已有分支则 `change attach`）占住它。地图/ticket 的**规划壳**可短暂在默认分支；若 ticket 要改 live specs，须在绑定分支做 Specs landing。
3. 解决它——按需深入（读相关 ticket 全文，调用 Notes 指定的 skill）。没把握时用 `llman-sdd-explore` 的逐问深挖。**不要**在未 binding 时改 `llmanspec/specs/**`。
4. 记录解决：把答案作为结论写入该 ticket 的 proposal，关闭它，并在地图的 Decisions-so-far 追加一行要点 + 指针。
5. 新增 ticket（先建再接线）；把答案让模糊点变清晰、升级成 ticket 的，从尚未清晰区移除。若答案揭示某 ticket 越过目的地，归入范围外而非在路径上解决。

## 输出
地图 change + 子决策 change 的依赖图（`llman sdd graph`）。路径清晰后建议进入 `llman-sdd-propose`（含 Branch binding → Specs landing，至 `readyToImplement=true`）把决策收拢为可实施计划。

## Context
- 先查状态再动手：change/spec 状态以 `llman sdd show/list/validate` 输出为准。
- 读 spec 全文前先用 `llman sdd context --task --paths` 定位相关 specs。

## Goal
- 本节命令达成一个可验证结果；结果路径与校验状态随报告输出。

## Constraints
- 遵守正文「硬约束/硬规则」，本节不复读。先判断变更规模选路径（triage）：行为合约变更走完整 SDD，实现层走 quick；不确定选完整 SDD（保守）。
- 改动保持最小；已知校验错误禁止强行继续。

## Workflow
- 每步以 `llman sdd` 命令结果为事实来源；改动工件后必跑 `llman sdd validate`。
- 命令细节见下方生成式命令参考或 `llman sdd <cmd> --help`。

## Decision Policy
- 高影响歧义先澄清再继续；事实自己查证，只有决策问用户。

## Output Contract
- 报告先给人读摘要（结论 / 风险 / 待决策），机器细节随后。

## Ethics Governance
- `ethics.risk_level`：low——仅读写本仓库与 `llmanspec/`，无外发动作；正文另有声明时从其声明。
- `ethics.prohibited_actions`：违反正文「硬约束」的动作；未经用户明确要求的 push / PR / 外部上传。
- `ethics.required_evidence`：结论须有命令输出或文件路径佐证；门禁状态以 `llman sdd validate` 为准。
- `ethics.refusal_contract`：门禁 CRITICAL 未清零 → 拒绝进入下一阶段；自修复达上限 → 报告 blocker。
- `ethics.escalation_policy`：改动 SDD 合约/模板或执行不可逆动作前，暂停并请用户确认。
