# Tasks

- [ ] 1.1 driver 骨架：`tools/eval/`（pyproject + cli + dataset + sandbox + agent + grade + report），或先落 `scripts/lab_eval_swebench.py`
- [ ] 1.2 dataset loader：固定 revision + 显式 instance ids；输出 run metadata
- [ ] 1.3 sandbox manager：Docker pull/run/exec/cp/timeout；每实例独立容器与清理
- [ ] 1.4 agent runner：容器内 `xylitol print`（隔离 HOME/config/session，`--no-trust`）
- [ ] 1.5 patch 收集：`git add -A && git diff --cached`；空 patch 也写 prediction
- [ ] 1.6 preds 序列化：`instance_id` / `model_name_or_path` / `model_patch`
- [ ] 2.1 official grader 调用：`swebench.harness.run_evaluation`；解析 `results.json`
- [ ] 2.2 run 目录与 `run.json` / `summary.md`：binary hash、config hash、dataset revision、harness 版本
- [ ] 2.3 失败分类：empty patch / apply error / test fail / infra error
- [ ] 3.1 Verified 5 冒烟：5/5 被 grade；重跑 resolved 集合可复现
- [ ] 3.2 单测：dataset 选择、patch 收集、preds 序列化、results 解析、report schema
- [ ] 4.1 M1 eval profile：`max_wall_seconds` / `max_turns` / `ask: disabled` / config 锁定
- [ ] 4.2 M1 成本/token 停止与 autosubmit（若需 runtime hook，另开 SDD change）
- [ ] 5.1 M2 扩到固定 Verified 50 + mini-SWE-agent 同口径 baseline
- [ ] 6.1 M3 Harbor `BaseInstalledAgent` + Terminal-Bench 10–20（独立 change）
