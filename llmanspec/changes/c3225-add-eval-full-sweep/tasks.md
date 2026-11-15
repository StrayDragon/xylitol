# Tasks

## 0. 方案与锁定

- [ ] 0.1 从 `deep-swe/tasks@v1.1` 选 5 个补充 task：覆盖 typical 未覆盖语言/仓库；镜像可拉取；verifier 可跑。
- [ ] 0.2 在 `tools/eval/model-lock.yaml` 补 `deepswe.full_ids` 与 `full_checksums`；保留 `typical_ids`。
- [ ] 0.3 在 `tools/eval/model-lock.yaml` 补 `terminal_bench.ids` 与 `checksums`（10 个 TB4 task）。
- [ ] 0.4 更新 `tools/eval/README.md`：DeepSWE 10 / TB4 10 口径、TB4 不跑 Pi、N=3/N=2。

## 1. 配方

- [ ] 1.1 新增 `just eval-full`：DeepSWE 10 × N=3 × xylitol/Pi；复用 `harbor_local.sh`，`-n 3`。
- [ ] 1.2 新增 `just eval-tb4`：TB4 10 × xylitol N=2；`-n 3`。
- [ ] 1.3 配方内固定 model lock、binary SHA、git commit 记录。

## 2. 聚合与报告

- [ ] 2.1 扩展 `summarize_harbor.py`：`--csv` 输出；per-task / per-scaffold 行。
- [ ] 2.2 增加无效 trial 计数：`exception_info` 分类、timeout、无改动 reward 0。
- [ ] 2.3 增加 N、commit/SHA、model、Harbor 版本元数据。
- [ ] 2.4 新增 `tools/eval/REPORT_TEMPLATE.md`：主表 + TB4 参考表 + 异常 + 成本 + 复跑。
- [ ] 2.5 在 README 写 SOP：固定输入、运行、无效 trial、聚合、发布、复核。

## 3. 执行（不在本提案阶段）

- [ ] 3.1 单 task 冒烟 5 个补充 DeepSWE task。
- [ ] 3.2 跑 DeepSWE 10 × N=3 × xylitol。
- [ ] 3.3 跑 DeepSWE 10 × N=3 × Pi。
- [ ] 3.4 跑 TB4 10 × xylitol N=2。
- [ ] 3.5 聚合 + 写报告；确认脚本可从 `jobs/` 重建。
- [ ] 3.6 同 commit + lock 重跑一次，确认聚合可复现。
- [ ] 3.7 更新 #21 与发布文档；不新增 live spec。

## 4. 验证

- [ ] 4.1 `just eval-test` 绿。
- [ ] 4.2 `llman-sdd validate c3225-add-eval-full-sweep --strict` 绿。
- [ ] 4.3 报告表格含 N、异常数、口径；CSV 与 Markdown 一致。
