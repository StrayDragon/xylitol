# Design：Full eval sweep + reproducible release report SOP

## 目标与边界

`tools/eval/` 是开发者工具链，不是产品功能。本 change 把 c3006 的 5-task typical
扩到 DeepSWE 10 + TB4 10，并把发布报告做成可复现 SOP。本文件只定方案，不进入实现。

## 当前状态（2026-11-15 复核）

- c3006 位于归档目录；Phase 0–3 交付 5-task DeepSWE typical、Pi 预装镜像、
  `just eval-typical`、`tools/eval/scripts/summarize_harbor.py`。
- `tools/eval/model-lock.yaml` 已声明 `deepswe.tasks: 10`、`typical_tasks: 5`、
  5 个 typical ids；其余 5 个 id 未钉。
- TB4 已移出典型集；`tools/eval/README.md` 规定只保留 xylitol N=2 历史记录，不跑 Pi。
- `summarize_harbor.py` 已算 pass@1、flip、cross-scaffold、errors、wall、cost；
  输出 Markdown + JSON，缺 CSV、N、无效 trial 计数、commit/SHA。
- `tools/eval/examples/eval-terminal-bench.yaml` 是 legacy 配置；Harbor 路径直接收
  task 路径，不读该配置。
- `just eval-typical` 跑 5 task × N=3；无 10-task / TB4 配方。
- `harbor_local.sh` 把并发上限压到 3；`xylitol_agent.py` 用 `max_steps_per_turn` 与
  `wall_seconds`。

## 决策

1. **DeepSWE 10 = 5 typical + 5 补充**。补充 task 的选择标准：
   - 覆盖 typical 未覆盖的语言/仓库类型；
   - 镜像可拉取、verifier 可跑、no-network 可用；
   - 单 task 冒烟 reward 可产出。
   task id 与 checksum 写入 `model-lock.yaml` 的 `deepswe.full_ids` / `full_checksums`；
   `typical_ids` 保留。
2. **TB4 10 = xylitol N=2 only**。不跑 Pi；floor 单列。理由：README 已规定；
   Pi 在 TB4 区分度低，跑 Pi 只增加成本与噪声。
3. **DeepSWE N=3；TB4 N=2**。同模型 `group/glm-5.3-flash`、同 1800s / 80 steps。
4. **报告口径**：分数只来自 Harbor verifier；pass@1 先按 task 聚合再取 scaffold 总体；
   flip rate 与 cross-scaffold gap 分开；TB4 单列。
5. **无效 trial**：Harbor `exception_info` 中的 quota/provider/sidecar/verifier infra
   错误标为无效并重跑；agent timeout 计为失败；无改动 reward 0 计为失败。
6. **聚合脚本**：扩展现有 `summarize_harbor.py`，不另起一套。新增 `--csv` 输出、
   无效 trial 计数、N、commit/SHA。
7. **配方**：新增 `just eval-full`（DeepSWE 10）与 `just eval-tb4`（TB4 10）；
   复用 `harbor_local.sh` 的并发上限 3。

## 运行矩阵

| 基准 | tasks | scaffold | N | 预算 | trials |
|---|---:|---|---:|---|---:|
| DeepSWE | 10 | xylitol | 3 | 1800s / 80 steps | 30 |
| DeepSWE | 10 | Pi | 3 | 1800s / 80 steps | 30 |
| TB4 | 10 | xylitol | 2 | 1800s / 80 steps | 20 |

合计 80 trials；按 `-n 3` 并发，最坏 wall 约 80 × 1800s / 3 ≈ 13.3h，加镜像与排队开销。
实际 wall 以 per-trial `agent_execution` 为准。

## 报告结构

```text
# Agent Eval 发布报告
- 口径：model / commit / binary SHA / Harbor / dataset revision / N / budget
- 主表：DeepSWE per-task pass@1（xylitol vs Pi）、overall、flip、cross-scaffold gap
- 参考表：TB4 per-task pass@1（xylitol N=2）、floor 标注
- 异常：无效 trial 数、重跑数、timeout 数
- 成本：token / cost / wall
- 复跑：命令 + lock 路径
```

## 文件落点

- `tools/eval/model-lock.yaml`：补 `deepswe.full_ids` / `full_checksums` 与
  `terminal_bench.ids` / `checksums`。
- `tools/eval/scripts/summarize_harbor.py`：CSV、无效 trial、N、commit/SHA。
- `tools/eval/REPORT_TEMPLATE.md`：报告模板。
- `tools/eval/README.md`：SOP 段 + `just eval-full` / `eval-tb4` 用法。
- `justfile`：新增两个配方。
- 不改 live spec；`needs_specs_change: false`。

## 风险

- 5 个补充 DeepSWE task 的镜像/verifier 稳定性未验证；先单 task 冒烟。
- TB4 20 个镜像 + 5 个补充 DeepSWE 镜像占磁盘；apply 前检查磁盘余量。
- N=3 噪声；报告必须给 N 与异常数。
- quota / 网络不稳定；无效 trial 规则先定。
- `summarize_harbor.py` 依赖 `result.json` 形状；Harbor 版本变化可能改字段。

## 不做什么

- 不做 CI / 观测 / 云。
- 不跑 TB4 Pi。
- 不自研 judge。
- 不改产品默认行为。
- 不在本 change 实现 Phase 4/5。
