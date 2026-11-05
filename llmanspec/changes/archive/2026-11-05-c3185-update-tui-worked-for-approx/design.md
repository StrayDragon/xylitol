# Design — 不足一秒统一显示 ~1s

`format_elapsed_secs(0)` → `~1s`；`format_duration` 只判负值，其余走同一函数。

- `Worked for` → `Worked for ~1s`。
- `Thought` → `Thought ~1s`。
- `secs < 0` → `None`。
