---
name: "llman-sdd-ff"
description: "Fast-forward：一次性创建规划壳（proposal/design/tasks），再 Branch binding + Specs landing。禁止写入 changes/<id>/specs/。"
metadata:
  version: "0.0.66"
  llman_sdd:
    bdd_mode: "on"
    skill_set: "optional"
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
6. **Specs landing**：在绑定分支编辑 live `llmanspec/specs/<capability>/spec.toon`（bdd-on 再加 `*.feature`）并 commit；无合约变更则 `skip_specs_landing: true`。
7. 校验：`llman sdd validate <id> --strict --no-interactive`。
8. 用 `llman sdd show <id> --json` 确认 `readyToImplement=true` 后，建议 `llman-sdd-apply`（不要在未就绪时建议 apply）。

## Git-native 生命周期（摘要）

勿混淆：**Skill 导航** ≠ **Git-native 生命周期**。全图见根 `AGENTS.md`「领域概念区分」或 `llman-sdd-propose` 内嵌全图。

硬规则：
1. **先** Branch binding（`change start` / `attach`）→ Full；**再** Specs landing（绑定分支编辑并 commit `llmanspec/specs/**`）。
2. 无 live 合约变更 → `skip_specs_landing: true`。apply 前须 `readyToImplement=true`。
3. **禁止**在默认分支 commit live specs；已 attach 勿重复 `start`。
行动前先阅读 `llmanspec/config.yaml`，并遵循其中的 `context` 与 `rules`（若有）。

常用命令：
- `llman sdd context --task "<描述>" --paths "<文件>"`（找相关 specs）。使用 pageindex agentic tree 后端（需 `LLMAN_SDD_INDEX_CHAT_MODEL`）。可用 `LLMAN_SDD_INDEX_BACKEND` 预设。
- `llman sdd list`（列出变更）
- `llman sdd list --specs`（列出 specs 及 purpose/scope 元数据）
- `llman sdd show <id>`（展示 change/spec；`--type change --output json` 含 `stage` / `specsLanded` / `skipSpecsLanding` / `readyToImplement`——apply 门禁看 `readyToImplement`，勿凭「完整工件」）
- `llman sdd validate <id>`（校验 change 或 spec）
- `llman sdd validate --all`（批量校验）
- `llman sdd index rebuild`（重建 pageindex 树索引——不需要模型）
- `llman sdd index check`（检查索引新鲜度）
- `llman sdd change new <id>`（仅创建规划壳草稿 `changes/<id>/proposal.md`；不写 live specs）
- `llman sdd change start <id> [--worktree]`（Designed→Full：干净树且在默认分支 → 创建 `sdd/<id>` 分支 + attach；仅 Branch binding，不等于 Specs landing，不等于可 apply）
- `llman sdd change attach <id> [--force]`（绑定已有非默认 feature 分支 + base SHA；拒绝绑到默认分支）
- `llman sdd change finalize <id> [--no-check]`（**推荐单 commit 收尾**——verify 之后；不要求干净树；门禁 + 自动 ff-merge + 文档改名）
- `llman sdd change checkpoint <id> [--no-check]`（干净工作区 + 归档前门禁；严格 sha = HEAD；finalize 的 fallback）
- `llman sdd change diff <id> [--export-patch <path>]`（只读 `base...HEAD` 审查/导出）
- `llman sdd change archive <id>`（封存：自动 ff-merge 到默认分支，再改名到 `changes/archive/`；单 commit 收尾优先 `finalize`）
- `llman sdd archive freeze [--before YYYY-MM-DD] [--keep-recent N] [--dry-run]`（冻结已归档目录）
- `llman sdd archive thaw [--change <id> ...] [--dest <path>]`（从冷备份恢复）
- `llman sdd graph [CHANGE] [--format mermaid] [--scope active|archived|all] [--depth N]`（生成变更依赖图）
- `llman sdd project migrate --kind spec-md2toon`（`.md`+fence → 独立 `.toon`；`partitioned` 已移除）
常见校验修复（TOON 独立文件 spec）：

1) 缺少校验作用域（`Spec valid_scope must not be empty`）：
Main spec 必须在 `.toon` 文档内携带非空的 `valid_scope`。
`llmanspec/specs/<feature-id>/spec.toon`：
```toon
kind: llman.sdd.spec
name: sample
purpose: "One-line overview."
valid_scope[1]: src
requirements[1]{req_id,title,statement}:
  r1,Title,System MUST do something.
scenarios[1]{req_id,id,given,when,then}:
  r1,happy,"",a trigger happens,the outcome is observed
```

2) 表格化行引号错误（"Expected N tabular row values, but got M"）：
值包含**空格**、逗号、冒号或方括号时，必须用双引号包裹。
```toon
# 错误：未加引号的空格值会被拆成多个值
r1,happy,"",a trigger happens,the outcome is observed

# 正确：多词值加引号
r1,happy,"","a trigger happens","the outcome is observed"
```

3) Git-native 护栏（配置了 `bdd:` 时采用 Partitioned SSOT）：
`spec.toon`=约束/不可执行场景；`*.feature`=可执行 GWT（`@req`）。
- **Branch binding** → **Specs landing**：先 `change start` / `attach`，再在绑定的非默认分支编辑 live 文件并 commit。规划壳可短暂在默认分支；**禁止**在默认分支改 live specs；**禁止**写 `changes/<id>/specs/`。
- apply 前须 `readyToImplement=true`（或 `skip_specs_landing`）。收尾（verify 后）优先 `change finalize`，勿在 propose/apply 中途 finalize。
- 勿使用 `change delta` / solidify / `*.feature.delta.toon`。配置了 `bdd:` 且空 requirements 又无 `.feature` = ERROR。

备注：
- 每个 spec 是一个独立的 `.toon` 文件；没有 Markdown 外壳，也没有 ```toon fence。
- `null` 表示可选字段缺失。
- 从旧版 `.md`+fence 迁移请使用 `llman sdd migrate`。

## Ethics Governance
- `ethics.risk_level`：按 `low|medium|high|critical` 标注风险等级。
- `ethics.prohibited_actions`：列出绝对禁止执行的动作。
- `ethics.required_evidence`：列出高影响输出前必须具备的证据。
- `ethics.refusal_contract`：定义何时拒答以及安全替代响应方式。
- `ethics.escalation_policy`：定义何时必须升级为用户确认/人工复核。
