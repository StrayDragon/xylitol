---
name: "llman-sdd-ff"
description: "一趟走完 propose 等价路径：规划文档 → 绑定分支 → 提交 specs。"
metadata:
  version: "0.7.5"
---

# LLMAN SDD Fast-Forward (FF)

快速走完 propose 等价路径：规划文档 → 绑定分支 → 提交 specs（至 specs-landed 门通过）。**不是**旧的 `changes/<id>/specs/` delta 模型。

## 硬约束

- 规划文档只写在 `llmanspec/changes/<id>/`（proposal/design/tasks）；specs 只写在绑定分支的 `llmanspec/specs/**`。
- **不得**创建 `llmanspec/changes/<id>/specs/`。
- `stage=full` 且 specs-landed 门通过（或 `needs_specs_change: false`）即可进 apply；verify/finalize 须 `readyToImplement=true`。

## 步骤

1. 取得一句话描述；change id 未给则推导并宣布（非阻塞，同 propose 规则）；确定受影响 capability。
2. 确保已 `llman-sdd init`（存在 `llmanspec/`）。
3. `llmanspec/changes/<id>/` 已存在：询问补齐或换 id；勿未确认就覆盖。
4. 建规划文档（可短暂在默认分支）：`llman-sdd change new <id>`（或手写）→ 充实 `proposal.md` → `design.md`（按需）→ `tasks.md`。
5. **绑定分支**：`llman-sdd change start <id>`（干净树 + 默认分支）或手动建分支后 `change attach <id>`。
6. **提交 specs**：在绑定分支编辑 `llmanspec/specs/<capability>.feature`（扁平，或目录主文件）并 commit；无合约变更设 `needs_specs_change: false`。
7. 校验：`llman-sdd validate <id> --strict`。
8. 用 `llman-sdd show <id> --output json` 确认 specs-landed 门绿（`specsLanded`/`needsSpecsChange`）后宜接 `llman-sdd-apply`（未提交前不得提示）。

## Git 分支生命周期（摘要）

**Skill 导航** ≠ **分支生命周期**。全图见根 `AGENTS.md` 或 `llman-sdd-propose` 内嵌图。

硬规则：
1. **先**绑定分支（`change start` / `attach`）→ full；**再**提交 specs（绑定分支上编辑并 commit `llmanspec/specs/**`）。
2. 无合约编辑 → `needs_specs_change: false`。`stage=full` 且 specs-landed 门通过即可进 apply；`readyToImplement=true`（全门绿）是 verify/finalize 前的完成信号。
3. 收口用 `change finalize`（自动提交 `archive(sdd): <id>`；`--no-commit` 跳过）。
4. **不得**在默认分支 commit specs；已 attach 不得重复 `start`。
5. worktree（可选）：`change start --worktree` 在独立 worktree 建分支、不动当前检出（`--base <branch>` 仅本地分支）记录分叉源；attach 缺省按运行时信号推导分叉源、偏离默认分支时输出 WARNING；finalize 目标即绑定分支时跳过合并就地收口，目标被其他 worktree 持有时自动在该 worktree 内执行（输出标注位置）。
> 命令细节用 `llman-sdd <cmd> --help` 查看；命令参考以 CLI 为准，skill 不内嵌命令表。
> 文中“规约”= 本项目 `llmanspec/specs/` 下的 `.feature` 文件；用 `llman-sdd list --specs` / `llman-sdd show <capability>` 查全文。
校验修复（单轨 feature-as-spec）：

1）缺头注释（`missing # capability: header comment`）：每个 capability `.feature`（`llmanspec/specs/<capability>.feature` 或目录内同名主文件）必须以下列注释开头：
```
# language: zh-CN
# capability: <capability>
# purpose: 一句话概述
# scope: src/
```

2）原生分层格式（`rule must carry an @req:<req_id> tag on the rule header`）：
- 规范样式只有一种：`@req:<id>` 挂在 `规则:` 块头标签,块内嵌套 `场景:`(假如/当/那么)是可执行示例——默认首选。
- 仅当需求无法程序化表达或暂不转写时才保留无嵌套场景的 `规则:`(裸规则):描述自由文本,不强制情态词;validate 以聚合计数提示,review `pending` 信号计量,specs-compact 负责压降。
- 历史标签 `@executable`/`@rule`/`@human`/`@manual` 不再使用、解析惰性;旧文件报结构问题时运行 `llman-sdd spec migrate-native` 迁移。
- 不在任何 `规则:` 内的顶层 `场景:` 是功能级示例:无规则句柄、不告警、不参与规则统计(Gherkin 原生语义)。

分支护栏：
- 先 `change start` / `attach` 绑定分支，再在绑定的非默认分支编辑 `.feature` 并 commit（提交 specs）。
- 锁定规则（报告制）：改/删既有 `规则:` 块只出 WARNING，不阻断 validate / finalize / `change diff`；报告按 `@req:<id>` 指明被改规则。控制点：git 分支对比 + `llman-sdd review` / `change diff`。旧锁定确认元数据（frontmatter `rules_touched` / `agent_acked`、`@agent` tag、`--yes` 确认语义）已全部删除，无别名无兼容层。
- `stage=full` 且 specs-landed 门通过（specsLanded ∨ `needs_specs_change: false`）即可进 apply；verify/finalize 须 `readyToImplement=true`（完成信号）。收口优先 `change finalize`。

## Ethics Governance
- `ethics.risk_level`：low——只读写本仓库与 `llmanspec/`，无外发动作；正文另有声明时按正文执行。
- `ethics.prohibited_actions`：违反正文硬约束的动作；用户未明确要求时，不得 push、发 PR 或向外部上传。
- `ethics.required_evidence`：结论必须给出命令输出或文件路径；门禁状态以 `llman-sdd validate` 输出为准。
- `ethics.refusal_contract`：门禁 CRITICAL 未清零时，不得进入下一阶段；自修复达到上限时，报告 blocker。
- `ethics.escalation_policy`：改动 SDD 合约、模板，或执行不可逆动作前，必须先取得用户确认。
