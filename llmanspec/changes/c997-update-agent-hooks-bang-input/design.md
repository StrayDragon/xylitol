# Design — c997

## 唯一发射点

`Driver::execute_bash` / agent 同路径 — **不是** TUI `parse_bang_command`。

Busy 硬拒（不当 bang）路径不发 hook（未进入 execute_bash）。

## input

明确 **out of scope**；另开 change 若需要。
