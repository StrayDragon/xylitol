---
name: "llman-sdd-validate"
description: "用户要求校验 change 或规格时使用。给出修复提示。不代替 apply 的实现循环。"
metadata:
  version: "0.11.0"
---

# LLMAN SDD 校验

校验 change/spec 格式与过期状态。

## 步骤
1. 单个：`llman-sdd validate <id>`；批量：`llman-sdd validate --all`（或 `--changes` / `--specs`）；CI/自动化用 `--strict`。
2. 校验失败时汇总错误，给出最小可执行修复法。

3. **Spec 校验**：
   - 在**绑定分支**上验证 `.feature` Gherkin 与 `@req` / 双写门禁；`.feature` 是 harness 权威——可执行 GWT 只在其中维护。
   - 生命周期门禁：`change start`（绑定分支）、`change archive`（收口；自动提交 `archive(sdd): <id>`，`--no-commit` 跳过，工作区须干净）/ `diff`（只读）。
   - `llman-sdd validate --specs` 做结构与合约门禁；配置 `specs.check_command` 时全量 harness 仅经显式 `--check` 执行（缺省跳过，仅结构/状态门），无占位符的命令每次调用至多执行一次。
   - `list --specs --json` 查看 `requirementCount` 与 `morphology`（requirementBoundCount / requirementUnboundCount / acceptanceCount / featureScenarioCount）。
   - change JSON 状态字段：`stage`（draft/designed/planned/full）/ `specsLanded` / `needsSpecsChange` / `readyToImplement`（`show --output json`）。


> 命令细节用 `llman-sdd <cmd> --help` 查看；命令参考以 CLI 为准，skill 不内嵌命令表。
> 文中“规约”= 本项目 `llmanspec/specs/` 下的 `.feature` 文件；用 `llman-sdd list --specs` / `llman-sdd show <capability>` 查全文。

校验修复（单轨 feature-as-spec）：

1）缺 `# purpose:` 头注释：每个 capability `.feature` 必须含非空 `# purpose:`；`功能:` 标题必须等于 discover 分配后的 spec id（`llman-sdd list --specs` 查看 id）。
```
# language: zh-CN
# purpose: 一句话概述

功能: <spec-id>
```

2）原生分层格式（`rule must carry an @req:<req_id> tag on the rule header`）：
- 规范样式只有一种：`@req:<id>` 挂在 `规则:` 块头标签,块内嵌套 `场景:`(假如/当/那么)是可执行示例——默认首选。
- 仅当需求无法程序化表达或暂不转写时才保留无嵌套场景的 `规则:`(裸规则):描述自由文本,不强制情态词;validate 以聚合计数提示,review `pending` 信号计量,specs-compact 负责压降。
- 历史标签 `@executable`/`@rule`/`@human`/`@manual` 不再使用、解析惰性;旧文件不再提供迁移命令,历史标签保持惰性。
- 不在任何 `规则:` 内的顶层 `场景:` 是功能级示例:无规则句柄、不告警、不参与规则统计(Gherkin 原生语义)。

分支护栏：
- 先 `change start` 绑定分支，再在绑定的非默认分支编辑 `.feature` 并 commit（提交 specs）。
- 锁定规则（报告制）：改/删既有 `规则:` 块只出 WARNING，不阻断 validate / finalize / `change diff`；报告按 `@req:<id>` 指明被改规则。控制点：git 分支对比 + `llman-sdd review` / `change diff`。旧锁定确认元数据（frontmatter `rules_touched` / `agent_acked`、`@agent` tag、`--yes` 确认语义）已全部删除，无别名无兼容层。
- `stage=full` 且 specs-landed 门通过（specsLanded ∨ `needs_specs_change: false`）即可进 apply；verify/finalize 须 `readyToImplement=true`（完成信号）。收口优先 `change archive`。

## Ethics Governance
- `ethics.risk_level`：low——只读写本仓库与 `llmanspec/`，无外发动作；正文另有声明时按正文执行。
- `ethics.prohibited_actions`：违反正文硬约束的动作；用户未明确要求时，不得 push、发 PR 或向外部上传。
- `ethics.required_evidence`：结论必须给出命令输出或文件路径；门禁状态以 `llman-sdd validate` 输出为准。
- `ethics.refusal_contract`：门禁 CRITICAL 未清零时，不得进入下一阶段；自修复达到上限时，报告 blocker。
- `ethics.escalation_policy`：改动 SDD 合约、模板，或执行不可逆动作前，必须先取得用户确认。
