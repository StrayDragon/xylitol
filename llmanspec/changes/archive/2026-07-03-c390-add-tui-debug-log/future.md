# c390 — Future（候选待办池）

> 来自 design.md §8「显式不做」。非承诺，触发后再开新 change。

## Deferred Items

- **CLI flag / settings 字段驱动**：若 env-only 在生产调试中门槛过高，考虑加 `--debug` 全局 flag 或 `settings.json` 的 `logging.{level,path}`。触发信号：用户反复抱怨「每次要设环境变量」。后续 change id 候选：`update-debug-log-activation`。
- **日志滚动**：单文件 append 长期会膨胀。考虑 `tracing-appender::RollingFileAppender`（按日，仿 codex `windows-sandbox-rs/src/logging.rs:50-62`，max 90 文件）。触发信号：log 文件 > 50MB 或跨天调试。候选：`update-debug-log-rotation`。
- **non-blocking writer**：若同步写在高频埋点（见下条启用后）下成为瓶颈，再上 `tracing-appender::non_blocking` + 进程退出前 `worker.flush()`。触发信号：profile 显示日志阻塞调用线程。候选：`update-debug-log-async`。
- **细粒度埋点**：render seam（pi `PI_DEBUG_REDRAW` 风格 redraw-reason log）、mpsc 流控、stream 状态机、cancel 路径。触发信号：出现需要这些路径才能定位的 bug。候选：`add-debug-log-instrumentation`。
- **TUI 内 debug 查看器**：参考 codex `log_db` SQLite Layer（结构化日志入 SQLite，bounded mpsc + 后台批量写）或 `tui-logger` 组件，做 in-app `:messages`/log panel。触发信号：不想 tail 文件、想交互过滤。候选：`add-tui-log-viewer`。
- **即时快照命令**：pi 的 `/debug` slash command——把当前渲染行 + 状态 dump 到文件，便于用户报 bug。候选：`add-tui-debug-snapshot`。
- **crash 强刷**：若未来改回 non_blocking，需配 abort/signal hook 在退出前 flush（参考 kimi-code `crash.ts`）。随 non-blocking 一起评估。

## Triggers to Reopen

- 上述任一触发信号出现。
- tracing 版本升级带来破坏性变更。
