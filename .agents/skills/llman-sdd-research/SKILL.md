---
name: "llman-sdd-research"
description: "以后台 agent 委托外部文献调研。当用户需要针对某个问题查阅官方文档/API/源码等一手资料、或想把阅读文献的活委托给后台 agent 时使用。"
metadata:
  version: "0.3.1"
---

# LLMAN SDD Research

启动一个**后台 agent** 做文献调研，这样你可以继续手头工作而它在读。

## Pipeline 位置

辅助工具，任意阶段可用。常见于 explore/wayfinder 阶段，为决策提供事实输入。产出回写 change 的 proposal「Further Notes」段，供后续阶段引用。

> 📍 这是独立可选 skill；调研产出供主流程的 explore/propose 消费。

## 职责

后台 agent 的工作：

1. 针对**一手资料**调研问题——官方文档、源码、spec、第一方 API——而非对它们的二手转述。把每个论断追溯到拥有它的源头。
2. 把发现写入单个 Markdown 文件，为每个论断标注来源引用。
3. 存放位置（仓库另有约定时优先遵循）：**默认** `llmanspec/changes/<current-change>/research/<topic>.md`（Change 文档，**不是** live specs）。仅当主题跨多个 change、归档后仍常引用时才写 `docs/research/`；**禁止**把单 change 选型/易腐深挖塞进 `docs/research/`。
4. **禁止**本 skill 直接编辑 `llmanspec/specs/**`。若调研表明必须改 MUST/SHALL → 建议 `llman-sdd-propose`（Branch binding → Specs landing）。

## 步骤

1. 明确要调研的问题（与用户确认；模糊时收窄到一个能被证实/证伪的问题）。
2. 用 Agent 工具 `subagent_type=general-purpose` + `run_in_background: true` 启动后台调研，prompt 含：
   - 问题陈述。
   - 要求只引一手资料，每条论断标注来源 URL/路径。
   - 输出文件路径（默认 `llmanspec/changes/<id>/research/<topic>.md`）。
   - 字数上限（建议：聚焦事实，散文式叙述 < 1500 词）。
3. 后台运行期间继续主流程工作；完成后收到通知。
4. 读取产出，把关键结论摘要回写到当前 change 的 `proposal.md`「Further Notes」段（附文件指针）。
5. 若调研揭示需要决策，建议进入 `llman-sdd-explore` 的逐问深挖分支。

## 与 wayfinder 的协作

`llman-sdd-wayfinder` 的查资料 ticket 委托本 skill 后台解决；解决后回写 ticket proposal 并在 map 的 Decisions-so-far 记一行要点。

> 命令细节用 `llman-sdd <cmd> --help` 查看；命令参考以 CLI 为准，skill 不内嵌命令表。
> 文中「规约」= 本项目 `llmanspec/specs/` 下的 `.feature` 文件；用 `llman-sdd list --specs` / `llman-sdd show <capability>` 查全文。

## Context
- 先查状态再动手：change/spec 状态以 `llman-sdd show/list/validate` 输出为准。
- 读 spec 全文前先用 `llman-sdd context --task --paths` 定位相关 specs。

## Goal
- 本节命令达成一个可验证结果；结果路径与校验状态随报告输出。

## Constraints
- 遵守正文「硬约束/硬规则」，本节不复读。先判断变更规模选路径（triage）：行为合约变更走完整 SDD，实现层走 quick；不确定选完整 SDD（保守）。
- 改动保持最小；已知校验错误禁止强行继续。

## Workflow
- 每步以 `llman-sdd` 命令结果为事实来源；改动工件后必跑 `llman-sdd validate`。
- 命令细节见下方生成式命令参考或 `llman-sdd <cmd> --help`。

## Decision Policy
- 高影响歧义先澄清再继续；事实自己查证，只有决策问用户。

## Output Contract
- 报告先给人读摘要（结论 / 风险 / 待决策），机器细节随后。

## Ethics Governance
- `ethics.risk_level`：low——仅读写本仓库与 `llmanspec/`，无外发动作；正文另有声明时从其声明。
- `ethics.prohibited_actions`：违反正文「硬约束」的动作；未经用户明确要求的 push / PR / 外部上传。
- `ethics.required_evidence`：结论须有命令输出或文件路径佐证；门禁状态以 `llman-sdd validate` 为准。
- `ethics.refusal_contract`：门禁 CRITICAL 未清零 → 拒绝进入下一阶段；自修复达上限 → 报告 blocker。
- `ethics.escalation_policy`：改动 SDD 合约/模板或执行不可逆动作前，暂停并请用户确认。
