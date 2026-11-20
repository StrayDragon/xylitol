# Tasks

## T1. invalid patch

- [ ] T1.1 `summarize_harbor.py` 识别缺失/空 `model.patch`，输出 `invalid`。
- [ ] T1.2 报告单列 invalid trials，不计入 pass@1。
- [ ] T1.3 用空 patch fixture 回归。

## T2. commit guard

- [ ] T2.1 在 `harbor_local.sh` 或 adapter 增加 collect 前 commit guard。
- [ ] T2.2 dirty/clean 两态测试；未完成时标 invalid。
- [ ] T2.3 报告标注 guard 是否触发。

## T3. step_limit 与 debug

- [ ] T3.1 `summarize_harbor.py` 识别 `[End] step_limit` 与 wall timeout。
- [ ] T3.2 README debug runbook：`view jobs` / `analyze` / `job resume` / `regrade` / per-trial artifacts / `xylitol tui --session`。
- [ ] T3.3 `just eval-test` 绿；报告/#21 更新。
