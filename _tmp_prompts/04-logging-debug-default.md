# 提示词：即时 debug 日志默认策略探查（用后即弃 / 供 c460）

## 背景

用户要求：debug 构建默认开即时 log（`tail -f`）；release 默认关。`src/app/cli/logging.rs` 现以 `XYLITOL_DEBUG` / `RUST_LOG` 为准。

## 任务

1. 读 `src/app/cli/logging.rs` 与 `src/app/tui/AGENTS.md`。
2. 给出最小改动方案：`cfg(debug_assertions)` 默认写 `~/.xylitol/logs/xylitol.log`（或现有路径）；release 保持现状。
3. 确认与 BDD/测试不抢全局 logger。
4. 把方案写进 `c460` 升格时的 tasks（本文件只探查，可不改代码）。

## 输出

`_tmp_prompts/03b-logging-findings.md`：结论 + 建议 patch 要点。
