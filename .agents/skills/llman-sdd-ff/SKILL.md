---
name: "llman-sdd-ff"
description: "Fast-forward：一次性创建规划壳（proposal/design/tasks），再 Branch binding + Specs landing。禁止写入 changes/<id>/specs/。"
metadata:
  version: "0.0.77"
---

# LLMAN SDD Fast-Forward (FF)

快速走完 propose 等价路径：规划壳 → Branch binding → Specs landing（至 `readyToImplement=true`）。**不是**旧的 `changes/<id>/specs/` delta 模型。

## 硬约束

- **规划壳**只写在 `llmanspec/changes/<id>/`（proposal/design/tasks）。
- Live 合约只写在绑定分支的 `llmanspec/specs/**`（Specs landing）。
- **禁止**创建 `llmanspec/changes/<id>/specs/` 或 `*.feature.delta.toon`。
- 进入 apply 前须 `readyToImplement=true`。

## 步骤

1. 询问用户：一句话描述、change id（或派生）、受影响 capability、确认最终 id。
2. 确保已 `llman sdd init`（存在 `llmanspec/`）。
3. 若 `llmanspec/changes/<id>/` 已存在：询问补齐或换 id；勿未确认就覆盖。
4. 创建**规划壳**（可短暂在默认分支）：
   - `llman sdd change new <id>`（或手写）→ 充实 `proposal.md`
   - `design.md`（按需）
   - `tasks.md`
5. **Branch binding**：`llman sdd change start <id>`（干净树 + 默认分支）或手动建分支后 `change attach <id>`。
6. **Specs landing**：在绑定分支编辑 live `llmanspec/specs/<capability>.feature`（扁平，或目录主文件）并 commit；无合约变更则 `needs_specs_change: false`。
7. 校验：`llman sdd validate <id> --strict --no-interactive`。
8. 用 `llman sdd show <id> --json` 确认 `readyToImplement=true` 后，建议 `llman-sdd-apply`（不要在未就绪时建议 apply）。

## Git-native 生命周期（摘要）

勿混淆：**Skill 导航** ≠ **Git-native 生命周期**。全图见根 `AGENTS.md`「领域概念区分」或 `llman-sdd-propose` 内嵌全图。

硬规则：
1. **先** Branch binding（`change start` / `attach`）→ Full；**再** Specs landing（绑定分支编辑并 commit `llmanspec/specs/**`）。
2. 无 live 合约变更 → `needs_specs_change: false`。apply 前须 `readyToImplement=true`。
3. 收口用 `change finalize`（自动提交 `archive(sdd): <id>`；`--no-commit` 可跳过）。`change checkpoint` 已移除。
4. **禁止**在默认分支 commit live specs；已 attach 勿重复 `start`。
> 命令细节用 `llman sdd <cmd> --help` 查看；命令参考以 CLI 为准，skill 不内嵌命令表（r139）。
校验修复（单轨 feature-as-spec）：

1）缺少头注释（`missing # capability: header comment`）：
每个 capability `.feature`（`llmanspec/specs/<capability>.feature` 或 `llmanspec/specs/<capability>/<capability>.feature`）必须以以下注释开头：
```
# language: zh-CN
# capability: <capability>
# purpose: 一句话概述
# scope: src/
```

2）tag 语法（`@human constraint scenario must carry an @req:<req_id> tag` / `orphan acceptance scenario`）：
- 规则：`@req:<id> @human` —— statement 放场景描述（须含 MUST/SHALL）。
- 验收：`@executable` 且至少一个 `@req:<id>` 挂到规则。
- `@manual` 须与 `@human` 同用；禁止 `@human` 与 `@executable` 同场景。

3）遗留 `spec.toon`（`legacy spec.toon found ... run ... toon2features`）：
运行 `llman sdd project migrate --kind toon2features --yes`，审阅 diff 后提交。

Git-native 护栏：
- **Branch binding** → **Specs landing**：先 `change start` / `attach`，再在绑定的非默认分支编辑 live `.feature` 并 commit。
- 锁定规则：修改/删除既有 `@human` 场景会触发门禁，除非 proposal frontmatter 的 `rules_touched` 列出被改动的 req-id。确认路径：finalize 交互一次 y/n 写回 `rules_touched`；`--yes` 只确认带 `@agent` 的规则（审计写入 `agent_acked`）；`rules_edit_acked` 已移除（r135/q9）。
- apply 前须 `readyToImplement=true`（或 `needs_specs_change: false`）。收尾优先 `change finalize`。
- 勿使用 `change delta` / solidify / `*.feature.delta.toon`。

## Ethics Governance
- `ethics.risk_level`：low——仅读写本仓库与 `llmanspec/`，无外发动作；正文另有声明时从其声明。
- `ethics.prohibited_actions`：违反正文「硬约束」的动作；未经用户明确要求的 push / PR / 外部上传。
- `ethics.required_evidence`：结论须有命令输出或文件路径佐证；门禁状态以 `llman sdd validate` 为准。
- `ethics.refusal_contract`：门禁 CRITICAL 未清零 → 拒绝进入下一阶段；自修复达上限 → 报告 blocker。
- `ethics.escalation_policy`：改动 SDD 合约/模板或执行不可逆动作前，暂停并请用户确认。
