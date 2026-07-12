# Tasks — c480-add-app-tui-input

## 1. Host 忙碌键位

- [x] 1.1 `HostSession`：busy Enter→pending steer；Alt+Enter→follow-up；Esc→pending abort
- [x] 1.2 `run_host_loop`：消费 pending，调 `Driver::steer` / `follow_up` / `abort`+`clear_queue`
- [x] 1.3 入队后刷新 `UiModel.queue` / footer；**chrome** 在 status 上方画 `Steering:` / `Follow-up:` + Alt+Up hint（**MUST NOT** scrollback `[steer]` 墙；SSOT `design/queue-steer.md`）
- [x] 1.4 Alt+Up：还原队列文本到 editor + `clear_queue(true, true)`
- [x] 1.4 单测：busy Enter/Alt+Enter/Esc

## 2. Slash MVP

- [x] 2.1 idle Enter：`/exit` 退出；`/model` 经 dispatch；未知 `/` 系统错误行
- [x] 2.2 单测：`/exit` 置 quit；未知 slash 不崩

## 3. 校验

- [x] 3.1 `just qa`
- [x] 3.2 `llman sdd validate c480-add-app-tui-input --strict --no-interactive`
