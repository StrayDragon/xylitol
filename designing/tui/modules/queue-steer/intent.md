# queue-steer

非空时在 scrollback 与 status 之间：`Steering:` / `Follow-up:`，末行 `↳ Alt+Up to edit all queued messages`。

- 忙碌 Enter=steer（不打断当前轮）；Alt+Enter=follow-up（空闲后新轮）。
- 注入历史后上行是普通 user 气泡；strip 消退内容仍留 transcript。
- 若同时有通知条：scrollback → 队列条 → toast → status。
- 禁止 footer/status 计数徽章；禁止写成 System / `[steer]` 墙。
