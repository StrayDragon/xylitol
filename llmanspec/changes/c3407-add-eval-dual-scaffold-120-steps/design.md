# c3407 design — 双 scaffold、120 步

## 目标与边界

把 actionlint 单 task baseline 从「xylitol only、80 步」改为「xylitol + Pi、120 步」，给两者同预算，便于下一次对比。计划文档只落 main，不绑定分支。

## 当前状态

- c3225 已归档：`llmanspec/changes/archive/2026-11-15-c3225-add-eval-full-sweep`。
- 120 步 + Pi 支持已在 `308b687` 落地；README 已在 `53f1d70` 对齐。
- `just eval-test` 22 passed；`just --dry-run eval-one xylitol` / `pi` 都显示 120。
- Pi 镜像与 `/tmp/deep-swe-agents/actionlint-action-pinning-lint` 已在上次清理删除。

## 决策

1. **step 上限 120**：80 步下 agent 在 Turn 79 触发 `[End] step_limit`，未提交；120 给同一 task 更多余量。
2. **双 scaffold**：`just eval-one` 增加 `scaffold` 参数；xylitol 走 `xylitol_agent:XylitolAgent`，Pi 走 `preinstalled_agents:PiPreinstalled`。
3. **Pi adapter 恢复**：`preinstalled_agents.py` 只做 `pi --version`，不重装。
4. **同预算**：两者都 `max_steps_per_turn=120`、`wall_seconds=1800`、N=3、同模型 `group/glm-5.3-flash`。
5. **不绑定分支**：本 change 只落 proposal/design/tasks；不 `change start`、不 `attach`。

## 运行矩阵

| task | scaffold | N | steps | wall |
|---|---|---:|---:|---:|
| `actionlint-action-pinning-lint` | xylitol | 3 | 120 | 1800 |
| `actionlint-action-pinning-lint` | Pi | 3 | 120 | 1800 |

## 风险与处置

| 风险 | 处置 |
|---|---|
| 120 步仍不够 | 记录 `[End] step_limit`；由 c3408 的 guardrail 标 invalid，不直接计 0 |
| Pi 镜像/任务目录缺失 | 下次跑前重拉 preinstalled 镜像并准备 `/tmp/deep-swe-agents/...` |
| 单 task 噪声 | N=3 只作回归 canary；不作能力结论 |
| 配额不足 | 本次只落计划，不跑模型；配额恢复后按矩阵跑 |

## 验证判据

- `just --dry-run eval-one xylitol` / `pi` 都含 `max_steps_per_turn=120`。
- `just eval-test` 22 passed。
- 实际跑完后，报告含两个 scaffold 的 pass@1、F2P/P2P、wall、commit/SHA。
