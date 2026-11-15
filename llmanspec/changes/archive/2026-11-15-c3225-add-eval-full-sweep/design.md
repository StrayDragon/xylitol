# Design：Single representative eval baseline

## 目标与边界

`tools/eval/` 是开发者工具链。本 change 把 c3225 从「DeepSWE 10 + TB4 10 全量 sweep」
收窄为「一个代表性 task 的回归 baseline」。全套留到 1.0.0 前另开 change。

## 当前状态

- c3006 位于归档目录：Harbor-first 5-task DeepSWE typical，xylitol vs Pi，N=3。
- 5-task baseline：xylitol 7/15，Pi 6/15；per-task 见 c3006 research。
- `just eval-typical` 跑 5 task × N=3；`eval-typical-both` 跑 xylitol + Pi。
- `tools/eval/model-lock.yaml` 有 `deepswe.tasks: 10`、`typical_tasks: 5`、5 个 typical ids。
- `tools/eval/examples/eval-terminal-bench.yaml` 是 TB4 legacy 配置；测试仍在读它。
- `summarize_harbor.py` 已能汇总 per-task pass@1 / flip / cross-scaffold / cost / wall。

## 决策

1. **单 task**：`actionlint-action-pinning-lint`。
   - 5-task baseline：xylitol 1/3，Pi 1/3；有 headroom。
   - 0 exceptions；无 timeout 噪声。
   - 真实 Go 仓库 + lint 修复 + 测试执行。
2. **单 scaffold**：xylitol only。Pi 对比留到 1.0.0 全套。
3. **N=3**：与既有 compare profile 一致；单 task 噪声大，只作回归 canary。
4. **预算**：1800s wall、80 steps；同模型 `group/glm-5.3-flash`。
5. **不锁 full 集**：`model-lock.yaml` 只写 `deepswe.baseline_id` + image digest；
   不写 `full_ids` / `full_checksums` / `terminal_bench.ids`。
6. **移除其他 eval**：
   - `justfile`：删除 `eval-typical` / `eval-typical-both`；新增 `eval-one`。
   - `tools/eval/model-lock.yaml`：删除 `typical_ids`、`tasks: 10`、`typical_tasks`、
     `terminal_bench` 段；保留 model/profile。
   - `tools/eval/examples/eval-terminal-bench.yaml` → `eval-one.yaml`；测试同步改。
   - `tools/eval/README.md`：删除 TB4 / 5-task 段；写 `eval-one` 用法。
   - 保留 `eval-smoke` 作为框架 canary。
7. **报告**：一页；`summarize_harbor.py --job baseline:xylitol=<jobs>` 输出。
   不新增 CSV / 模板 / SOP。

## 运行矩阵

| task | scaffold | N | 预算 | trials |
|---|---|---:|---|---:|
| actionlint-action-pinning-lint | xylitol | 3 | 1800s / 80 steps | 3 |

3 trials；`-n 3` 并发，最坏 wall 约 1800s + 镜像/排队开销。

## 文件落点

- `justfile`：`eval-one`；删 `eval-typical` / `eval-typical-both`。
- `tools/eval/model-lock.yaml`：`deepswe.baseline_id`；删 `typical_ids` / `terminal_bench`。
- `tools/eval/examples/eval-one.yaml`：新单 task 配置；删 `eval-terminal-bench.yaml`。
- `tools/eval/tests/`：`test_config` / `test_lock` / `test_cli_dryrun` / `test_agent` 改用 `eval-one.yaml`。
- `tools/eval/README.md`：单 task 用法；删 TB4 / 5-task 段。
- 不改 live spec；`needs_specs_change: false`。

## 风险

- 单 task N=3 噪声大；只作 canary。
- 移除 5-task / TB4 后，历史报告仍可读；日常入口变少。
- `summarize_harbor.py` 依赖 `result.json` 形状；Harbor 版本变化可能改字段。
- `actionlint` 可能随镜像/verifier 变化；记录 image digest。

## 不做什么

- 不做全套 sweep；不做 TB4；不跑 Pi。
- 不做 CSV / 模板 / SOP / CI / 云。
- 不锁 full ids / checksums。
