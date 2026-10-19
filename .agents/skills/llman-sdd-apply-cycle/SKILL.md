---
name: "llman-sdd-apply-cycle"
description: "单 change 一轮到底：实施→测试→校验→verify→归档提交。仅手动触发，agent 不得自动调用。"
metadata:
  version: "0.10.0"
disable-model-invocation: true
---

# LLMAN SDD Apply Cycle

单 change 一轮到底（手动）。须已绑定分支且 specs-landed 门通过（`specsLanded ∨ needsSpecsChange=false`）；`readyToImplement=true`（完成信号）收拢循环。

**仅手动触发**：`/skill:llman-sdd-apply-cycle <change-id>`

## 工作流

### 0) 门禁 + 状态
```bash
llman-sdd show <change-id> --output json --type change
```
> 阶段判定用 `stage` / `readyToImplement` 字段；完整判定表见 llman-sdd-apply。

- 须在绑定的非默认分支上。
- specs-landed 门未过 → STOP（先提交 specs 或 `needs_specs_change: false`）。已绿但 `readyToImplement=false` → 正常：tasks 待完成，继续实施；**仅当 `readyToImplement=true` 才 finalize**。
- 进度以 `tasks.md` checkbox 为准（或 `llman-sdd list` 任务计数）；实现时仍须读 `tasks.md`、proposal/design 与绑定分支上的 `llmanspec/specs/**`（唯一事实来源）。

### 1) 循环：实施 → 测试
对每个未完成 task：
1. 按 task + specs 实现（最小改动）
2. task 文本写明验证命令时运行之
3. 失败修复重试（自修复预算同 `llman-sdd-apply`：上限 8 轮）
4. 勾选 `tasks.md` 为 `[x]`

### 2) 校验
```bash
llman-sdd validate <change-id> --strict
```
失败修复重试（上限 8 轮）。

### 3) Verify（推荐）
优先跑 `llman-sdd-verify`（或等效双轴自检）。有 CRITICAL → STOP，勿归档。

### 4) 归档 + 提交
```bash
llman-sdd change finalize <change-id>
```
工作区可脏；自动合并（默认 squash）+ 改名 + **自动提交** `archive(sdd): <change-id>` 单进程完成。`--no-commit` 跳过自动提交（手动/CI 历史）——此时自行 `git add -A && git commit -m "archive(sdd): <change-id>"`。普通 `change archive` 保留为 fallback。

### 5) 可选清理
```bash
git branch -D <feature-branch>   # squash 后分支不再是 main 祖先，-d 会被拒
```
push / PR 仅当用户明确要求。

## 硬约束
- **不得问“是否继续”**——除非 blocker，一路到底。
- **不得切换**其他 change，直到本 change 归档并提交。
- **不得**写 `changes/<id>/specs/`；**不得默认 push/PR**。

## Ethics Governance
- `ethics.risk_level`：low——只读写本仓库与 `llmanspec/`，无外发动作；正文另有声明时按正文执行。
- `ethics.prohibited_actions`：违反正文硬约束的动作；用户未明确要求时，不得 push、发 PR 或向外部上传。
- `ethics.required_evidence`：结论必须给出命令输出或文件路径；门禁状态以 `llman-sdd validate` 输出为准。
- `ethics.refusal_contract`：门禁 CRITICAL 未清零时，不得进入下一阶段；自修复达到上限时，报告 blocker。
- `ethics.escalation_policy`：改动 SDD 合约、模板，或执行不可逆动作前，必须先取得用户确认。

### Ethics Governance 覆写
本技能含 finalize 合并等不可逆动作,以下键覆写共享单元:

- `ethics.risk_level`: medium。
- `ethics.required_evidence`: 另须 `readyToImplement=true` 与 tasks 全勾。
- `ethics.escalation_policy`: 归档前暂停并请用户确认。

> 命令细节用 `llman-sdd <cmd> --help` 查看；命令参考以 CLI 为准，skill 不内嵌命令表。
> 文中“规约”= 本项目 `llmanspec/specs/` 下的 `.feature` 文件；用 `llman-sdd list --specs` / `llman-sdd show <capability>` 查全文。
