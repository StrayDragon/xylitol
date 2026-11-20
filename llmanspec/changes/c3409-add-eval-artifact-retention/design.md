# c3409 design — 保留与清理

## 目标

把「删哪些 job/tmp/镜像、保留哪些」写成可重复脚本与规则，避免再次手工删掉诊断证据。

## 当前状态

- `jobs/one-actionlint-action-pinning-lint` 已删除。
- `/tmp/deep-swe`、`/tmp/deep-swe-agents`、`/tmp/tb4` 已删除。
- 约 36GB 未用镜像已删除；保留 actionlint task 镜像 + egress sidecar。
- 保留：最新报告 `tools/eval/reports/actionlint-baseline.md`、`target/eval-bin`、`/tmp/harbor-venv`、`/tmp/magpie-harbor.env`。

## 决策

1. **保留最新报告 + 最新 job**：默认 14 天；最新报告不删。
2. **镜像保留**：actionlint task 镜像 + egress sidecar 常驻；Pi 镜像按需，删除后下次重拉。
3. **脚本安全**：默认 dry-run；`--apply` 才删；显式路径校验；不 `rm -rf` 未验证路径。
4. **不自动定时**：手动跑；避免在跑 eval 时删正在使用的 job/image。
5. **不删 binary/Harbor venv/env**：这些是复用 setup。

## 风险与处置

| 风险 | 处置 |
|---|---|
| 误删正在使用的 job/image | dry-run + 显式 `--apply` + 路径校验 |
| 删掉诊断证据 | 最新 job 保留 14 天；报告不删 |
| Pi 镜像删除后缺镜像 | README 注明下次重拉 |
| 磁盘不足 | 默认 14 天，可配置；先 dry-run 看体积 |

## 验证判据

- dry-run 只列过期项，不删除。
- `--apply` 后最新报告、binary、Harbor venv/env 保留。
- README 有保留策略；`just eval-test` 绿。
