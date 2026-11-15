# Tasks

## 0. 锁定单 task

- [x] 0.1 确认 `actionlint-action-pinning-lint`；记录 image digest 与 verifier 版本。
- [x] 0.2 `model-lock.yaml` 改为 `deepswe.baseline_id`；删除 `typical_ids`、`tasks: 10`、`typical_tasks`、`terminal_bench` 段。
- [x] 0.3 新增 `examples/eval-one.yaml`；删除 `examples/eval-terminal-bench.yaml`。
- [x] 0.4 改 `tests/test_config.py` / `test_lock.py` / `test_cli_dryrun.py` / `test_agent.py` 读 `eval-one.yaml`。
- [x] 0.5 更新 `README.md`：单 task / xylitol only / N=3；删除 TB4 与 5-task 段。

## 1. 配方

- [x] 1.1 新增 `just eval-one`：`actionlint-action-pinning-lint` × N=3 × xylitol；`-n 3`。
- [x] 1.2 删除 `eval-typical` / `eval-typical-both`。
- [x] 1.3 保留 `eval-smoke`；确认它仍可跑。

## 2. baseline

- [x] 2.1 跑 `just eval-one`；记录 `jobs/` 路径。
- [x] 2.2 用 `summarize_harbor.py` 生成一页 baseline：task、N、pass@1、wall、cost、commit/SHA、Harbor 版本。
- [x] 2.3 同 commit + lock 重跑一次，确认结果可由脚本重建。
- [x] 2.4 更新 #21 与 README 的 baseline 指针；不新增 live spec。

## 3. 验证

- [x] 3.1 `just eval-test` 绿。
- [x] 3.2 `llman-sdd validate c3225-add-eval-full-sweep` 绿。
- [x] 3.3 `just eval-smoke --check` 绿（不调用模型）。
