---
name: "llman-sdd-apply"
description: "实施一个 llman SDD 变更的 tasks，并同步更新 tasks.md 勾选状态。"
metadata:
  version: "0.0.56"
---

# LLMAN SDD Apply

使用此 skill 按顺序完成 `llmanspec/changes/<id>/tasks.md`，直到完成或受阻。

## 步骤
1. 使用 `llman sdd context --task "<proposal 中的目标>" --paths "<specs 中的 scope>"` 确认相关 specs。
   - 如果 context 不可用，运行 `llman sdd index rebuild`（默认 `pageindex` 树索引，无需模型）后重试；rag backend 则加 `--backend rag`。
2. 选择变更 id：
   - 若已提供，直接使用。
   - 否则先从上下文推断；若不明确，运行 `llman sdd list --json` 并让用户选择。
   - 始终说明："使用变更：<id>"，并告知如何覆盖。
2. 检查前置条件（权威阶段守卫）：
   - 从权威来源读取变更阶段：
     ```bash
     stage=$(llman sdd show <id> --json --type change | jq -r .stage)
     ```
     （若无 `jq`，可用任意工具从 JSON 中解析 `stage` 值。）
   - 若 `stage` 为 `draft`，变更尚未准备好被实现 → 必须停止并给出守卫提示：
     `draft`："变更 <id> 是 draft 提案（仅 proposal.md），尚未准备好被实现。请先用 llman-sdd-continue <id> 把它长大到至少 `spec` 阶段（proposal → specs → tasks）。"
   - `specified`、`designed`、`full` 阶段均可被实现（存在 tasks.md 即可 apply），继续。
3. 阅读上下文文件（视情况而定）：
   - `llmanspec/changes/<id>/proposal.md`
   - `llmanspec/changes/<id>/design.md`（如存在）
   - `llmanspec/changes/<id>/tasks.md`
   - `llmanspec/changes/<id>/specs/**`
4. 展示状态：
   - 进度："N/M tasks complete"
   - 接下来 1–3 个未完成任务（简短概览）
5. 按顺序实施 tasks：
   - 改动保持最小并严格围绕当前任务
   - 完成一项任务后立刻更新 checkbox（`- [ ]` → `- [x]`）
   - 若任务不明确、遇到阻塞、或发现 specs/design 与现实不一致，必须 STOP 并询问用户下一步。

6. **BDD 回归**:
   - 每完成一个 task，运行关联的 BDD 测试确保不回退:
     `cargo test --test bdd -- --test-threads=1`
   - 如有 scenario 由 PASS 变 FAIL，立即停止并报告

7. 在完成（或暂停）时运行校验：
   ```bash
   llman sdd validate <id> --strict --no-interactive
   ```
   - 若校验无误，建议运行 `llman-sdd-verify`，然后执行归档：`llman sdd archive run <id>`。

在执行之前，请先阅读 `llmanspec/config.yaml`，若其中包含 `context` 与 `rules` 请遵循。

常用命令：
- `llman sdd context --task "<description>" --paths "<files>"`（获取相关 specs）。使用 pageindex agentic 树检索后端（需配置 `LLMAN_SDD_INDEX_CHAT_MODEL`）。可用 `LLMAN_SDD_INDEX_BACKEND` 预设。
- `llman sdd list`（列出变更）
- `llman sdd list --specs`（列出 specs，含 purpose/scope 元数据）
- `llman sdd show <id>`（查看 change/spec）
- `llman sdd validate <id>`（校验变更或 spec）
- `llman sdd validate --all`（批量校验）
- `llman sdd index rebuild`（重建 pageindex 树索引——无需模型）
- `llman sdd index check`（检查索引新鲜度）
- `llman sdd archive run <id>`（归档变更）
- `llman sdd archive freeze [--before YYYY-MM-DD] [--keep-recent N] [--dry-run]`（冻结归档目录）
- `llman sdd archive thaw [--change <id> ...] [--dest <path>]`（解冻归档）
- `llman sdd graph [CHANGE] [--format mermaid] [--scope active|archived|all] [--depth N]`（生成变更依赖图）


## Context
- 执行前先确认当前 change/spec 状态。
- 优先使用 `llman sdd context --task --paths` 获取相关 specs，而非全量读取或猜测。

## Goal
- 明确本次命令/skill 要达成的可验证结果。

## Constraints
- 变更保持最小化且范围明确。
- 标识符或意图不明确时禁止猜测。
- 在读取 spec 全文前，先使用 `llman sdd context --task --paths` 获取相关 specs。
- 判断变更规模后选择路径：行为合约变更走完整 SDD 流程，实现变更走快速路径。

## Workflow
- 以 `llman sdd` 命令结果为事实来源。
- 涉及文件/规范变更时执行校验。
- 首选 `llman sdd context` 获取相关 specs，而非全量读取或猜测。
- 当 context 不可用时，按错误提示处理（重建 index 或降级到 `list --specs --json`）。

## Decision Policy
- 高影响歧义必须先澄清。
- 已知校验错误下禁止强行继续。

## Output Contract
- 汇总已执行动作。
- 给出结果路径与校验状态。

## Ethics Governance
- `ethics.risk_level`：按 `low|medium|high|critical` 标注风险等级。
- `ethics.prohibited_actions`：列出绝对禁止执行的动作。
- `ethics.required_evidence`：列出高影响输出前必须具备的证据。
- `ethics.refusal_contract`：定义何时拒答以及安全替代响应方式。
- `ethics.escalation_policy`：定义何时必须升级为用户确认/人工复核。
