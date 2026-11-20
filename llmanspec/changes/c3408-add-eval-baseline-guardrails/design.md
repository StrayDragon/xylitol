# c3408 design — guardrails 与 debug

## 目标

把「没有 patch」「step_limit」「wall timeout」从「0 分」中分离出来，并给出可照做的 Harbor 调试路径。

## 证据

- 3 个 trial 都触发 `[End] step_limit`；Turn 79。
- `artifacts/manifest.json`：`model.patch` 状态 `skipped`。
- `verifier/ctrf.json`：200 tests，145 passed，55 failed（F2P）。
- `verifier/reward.json`：`reward: 0`；0 exceptions；wall 24–29 min，未到 1800s。
- `trial.log`：collect hook `git diff --binary base HEAD`。

## 决策

1. **invalid patch 语义**：`model.patch` 缺失或为空 → trial 标 `invalid`；报告单列，不计入 pass@1。
2. **commit guard**：在 collect 前检查工作树；dirty 时 `git add -A && git commit -m "eval: collect"`；若 agent 明确未完成，则标 invalid 而不是提交半成品。
3. **step_limit 识别**：从 `agent/xylitol.txt` 末尾读 `[End] step_limit`；与 wall timeout 分开报告。
4. **debug runbook**：Harbor 的 `view jobs` / `analyze` / `job resume` / `job regrade`；per-trial `agent`、`verifier`、`artifacts`、`trial.log`；`xylitol tui --session` 续看。
5. **不改 task/verifier**：优先在 wrapper/summarizer 层实现，避免改第三方 task.toml。

## 风险与处置

| 风险 | 处置 |
|---|---|
| commit guard 提交半成品 | 先判断 agent 是否已结束；未完成则 invalid |
| invalid 改变历史对比 | 旧 0/3 保留为 invalid，不重算 |
| 日志已清理 | 用 fixture 或下一次 run 回归 |
| 改 collect hook 触及 task | 只在 wrapper/summarizer 层实现 |

## 验证判据

- 空 patch fixture → `invalid`，不是 0。
- dirty fixture → collect 前有 commit 或明确 invalid。
- `[End] step_limit` 被识别并写入报告。
- `just eval-test` 绿。
