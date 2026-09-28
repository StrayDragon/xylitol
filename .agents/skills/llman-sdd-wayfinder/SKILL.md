---
name: "llman-sdd-wayfinder"
description: "把超出单会话的大型模糊工作拆成决策地图，逐个解决直到路径清晰。仅手动触发。"
metadata:
  version: "0.5.1"
disable-model-invocation: true
---

# LLMAN SDD Wayfinder

一个又大又乱的工作来了——大到单个 agent 会话装不下，从现在到**目的地**的路还看不见。这个 skill 不急着动手，先把路找出来：把路径画成 change 依赖图（`llman-sdd graph`），每个子工作（ticket）解决一个**决策**而非交付代码，逐个解决直到路径清晰。

## Pipeline 位置

辅助工具，用于主 pipeline 之前的**大型工作预规划**。地图清晰后 → `llman-sdd-propose` 收拢为可实施计划。

## 核心原则

- **只规划，不动手**：每个 ticket 解决一个决策；地图完成于「路径清晰、无决策遗留」。想直接开干的冲动，通常是到了地图边界、该交接的信号。
- **用名字指代**：给人看的叙述里用 ticket 标题指代，MUST NOT 用裸 id/编号。
- **单会话单 ticket**（查资料 ticket 例外）。

## 地图结构

地图本身是一个 change（总纲 proposal），子决策是 `depends_on` 的子 change。用 `llman-sdd graph <map-id> --scope active` 可视化当前**可着手项**。

地图的 `proposal.md` 结构：

```markdown
## Destination（目的地）
<走到终点是什么样——spec/决策/变更。一两行。>

## Notes（备注）
<领域；每会话应查的 skill；常设偏好>

## Decisions so far（已定的决策）
<!-- 索引：每个已关闭 ticket 一行，结论要点 + 链接 -->

## Not yet specified（尚未清晰区）
<!-- 能预见但还说不清成 ticket 的；随推进逐渐变清晰 -->

## Out of scope（范围外）
<!-- 超出目的地的；关闭的 ticket，永不复活 -->
```

## Ticket 类型

每个 ticket 是一个子 change，带 `wayfinder:<type>` 标注（写在 proposal 标题或 frontmatter）：

- **Research（查资料，agent 自跑）**：读文档/API/本地资源，查出决策在等的事实。委托 `llman-sdd-research` 后台解决。
- **Prototype（做原型，需人参与）**：用廉价粗糙的可运行物（throwaway 小程序或 UI 变体）把讨论具象化。
- **逐问深挖（需人参与）**：用 `llman-sdd-explore` 的逐问深挖分支一问一答走清。**默认类型**。
- **Task（杂活，人或 agent）**：决策前必须先做的手动工作（注册服务、迁数据以看清形状）。

## 尚未清晰区

地图**故意**不完整。判断一个点该不该现在立 ticket，只看一条：**现在能不能把问题说清楚**（不是能不能回答）。
- 能说清 → 立 ticket（即使暂时被挡）。
- 说不清 → 写进**尚未清晰区**（一团模糊日后可能变成多个 ticket，也可能一个都不变）。

## 步骤

### 画地图
1. **命名目的地**：用逐问深挖钉死这趟地图要通往哪里。
2. **广度优先扫可着手项**：再次逐问深挖，扇开而非深挖一条，浮出开放决策与现在能迈的第一步。若**没有模糊点浮出**——路径已清晰、一个会话能装下——不需要地图，停下问用户想怎么做。
3. **创建地图**（总纲 change）：`llman-sdd change new <map-id>`，填 Destination/Notes，Decisions-so-far 留空，模糊点写进尚未清晰区。
4. **创建现在能说清的 ticket** 为子 change，再用 `llman-sdd graph` 接依赖边（先有 id 才能互引）。
5. 为每个查资料 ticket 启动 `llman-sdd-research` 后台 subagent。
6. 停——画图是单个会话的活，不要顺手解决任何决策。

### 推进地图
1. 加载地图（低分辨率视图，不用读每个 ticket 全文）。
2. 选 ticket（用户指定或取可着手项第一个），先绑定分支（`change start` 或已有分支 `change attach`）占住它。规划文档可短暂在默认分支；ticket 要改 specs 时须在绑定分支落地。
3. 解决它——按需深入（读相关 ticket 全文，调用 Notes 指定的 skill）；没把握时用逐问深挖。**不要**在未绑定时改 `llmanspec/specs/**`。
4. 记录解决：答案写入该 ticket 的 proposal 并关闭它，在地图 Decisions-so-far 追加一行要点 + 指针。
5. 新增 ticket（先建再接线）；答案让模糊点变清晰的，升级为 ticket 并移出尚未清晰区；答案揭示某 ticket 越过目的地的，归入范围外而非在路径上解决。

## 输出
地图 change + 子决策 change 的依赖图（`llman-sdd graph`）。路径清晰后建议 `llman-sdd-propose` 收拢为可实施计划。

> 命令细节用 `llman-sdd <cmd> --help` 查看；命令参考以 CLI 为准，skill 不内嵌命令表。
> 文中「规约」= 本项目 `llmanspec/specs/` 下的 `.feature` 文件；用 `llman-sdd list --specs` / `llman-sdd show <capability>` 查全文。

## Context
- 先查状态再动手：change/spec 状态以 `llman-sdd show/list/validate` 输出为准；读 spec 全文前先用 `llman-sdd context --task --paths` 定位。

## Goal
- 达成一个可验证结果；报告附结果路径与校验状态。

## Constraints
- 遵守正文硬约束（不复读）。先判断规模选路径：合约变更走完整 SDD，实现层走 quick；不确定选完整 SDD。改动最小；已知校验错误禁止强行继续。

## Workflow
- 每步以 `llman-sdd` 命令结果为事实来源；改动工件后必跑 `llman-sdd validate`；命令细节见 `llman-sdd <cmd> --help`。

## Decision Policy
- 高影响歧义先澄清再继续；事实自己查证，只有决策问用户。

## Output Contract
- 先给人读摘要（结论 / 风险 / 待决策），机器细节随后。

## Ethics Governance
- `ethics.risk_level`：low——仅读写本仓库与 `llmanspec/`，无外发动作；正文另有声明时从其声明。
- `ethics.prohibited_actions`：违反正文「硬约束」的动作；未经用户明确要求的 push / PR / 外部上传。
- `ethics.required_evidence`：结论须有命令输出或文件路径佐证；门禁状态以 `llman-sdd validate` 为准。
- `ethics.refusal_contract`：门禁 CRITICAL 未清零 → 拒绝进入下一阶段；自修复达上限 → 报告 blocker。
- `ethics.escalation_policy`：改动 SDD 合约/模板或执行不可逆动作前，暂停并请用户确认。
