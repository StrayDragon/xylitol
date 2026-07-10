# Design — c460 app-tui-host

## 决策

### D1 — Host 驱动，不调 `TUI::start()`

产品路径自建 async 循环：`tokio::select!` 合流 crossterm 输入 / resize / idle tick / 取消信号。引擎只收 `dispatch_event` / `idle_tick` / `request_render` / `try_render`。

### D2 — 可测的事件源

抽出 `HostEvent` + `HostSession`（对 `TUI` 的一步推进），生产用 crossterm 轮询，测试用注入队列。难逻辑（min-size、restore 标志）放纯函数 / 小结构，不绑真 TTY。

### D3 — 终端 restore

`TerminalGuard`：`start` 时 `terminal.start()`，`Drop` 调 `stop()`；另装 panic hook（仅靠 Drop 不够，abort/panic unwind 路径）。Unix：`SIGTERM`/`SIGHUP` 设退出标志并尽力 `disable_raw_mode`。

### D4 — 最小尺寸

`cols < 40 || rows < 6` → 渲染单行提示组件，不跑完整 shell，不 panic。尺寸恢复后切回空壳。

### D5 — 空壳组件

本面 `Shell`：`Container` 或手写 `Component` — transcript 占位 Text、Editor（可空）、footer Text。不接 Driver 事件（c465）。

### D6 — 日志

`init_logging`：在 env 未开时，`cfg(debug_assertions)` 默认写文件（见 `_tmp_prompts/03b-logging-findings.md`）。

### D7 — crossterm 依赖

主 crate `tui` feature 引入 `crossterm`（与 xylitol-tui 同版本），host 轮询 `Event`；不把 tokio 绑进包。
