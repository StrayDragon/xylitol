# Tasks

## T2. 清理脚本

- [ ] T2.1 写 `tools/eval/scripts/cleanup_artifacts.sh`：dry-run 默认，`--apply` 才删。
- [ ] T2.2 支持 jobs/tmp/images 三类；路径校验；不删最新报告。
- [ ] T2.3 用当前状态 dry-run，确认只列过期项。

## T3. 保留策略

- [ ] T3.1 README 写保留策略：最新报告 + 最新 job 14 天；actionlint 镜像 + egress 常驻；Pi 按需。
- [ ] T3.2 记录 2026-11-15 清理结果。
- [ ] T3.3 `--apply` 后验证最新报告、binary、Harbor venv/env 保留。
- [ ] T3.4 `just eval-test` 绿。
