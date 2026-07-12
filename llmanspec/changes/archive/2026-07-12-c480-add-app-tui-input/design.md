# design — c480 app-tui-input

## 问题

产品 Host 只有 idle Enter→submit 与 Ctrl+C / 双 Esc stub；忙碌键位与 slash 未接 Driver。

## 决策

### A. 忙碌键位路由

| 键 | 条件 | 行为 |
|---|---|---|
| Enter | busy + editor 非空 + 树关 | `steer(text)`；清空 editor；刷新 queue 徽章；**chrome** 画 `Steering:`（见 `design/queue-steer.md`） |
| Alt+Enter | 同上 | `follow_up(text)`；清空 editor；chrome 画 `Follow-up:` |
| Alt+Up | busy + 队列非空 | 队列文本还原 editor；`clear_queue(true, true)` |
| Esc | busy（树关） | `abort()` + `clear_queue(true, false)`；**不**开树 |
| Esc | idle 双击窗 | 既有 stub 树（不变） |
| Ctrl+C | 任意 | 既有：非空清 / 空退出 |

实现落在 `HostSession::step`（可测）+ `run_host_loop` 消费 pending 队列调用 Driver。

### B. Slash MVP

Idle Enter 且 trim 以 `/` 开头：
- `/exit` → `request_quit` + finish_inline（不经 dispatch Prompt）
- `/model` → 无参则 `CycleModel` 或列出；有参则 `SetModel`（经 `dispatch`）
- 其它 → `UiEntry::System` 短错误

**不做** Command plate overlay（out of scope）。

### C. 与 InputListener

Ctrl+C / Esc-树 仍走 listener；忙碌 Esc abort 在 host `try_*` 中先于 `dispatch_event` 消费（与 idle Enter 同形）。

## 非目标

c492 bash、Completion popup、真 travel、approve 工具 UI。
