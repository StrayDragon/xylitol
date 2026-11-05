# Design — 固定 status/spinner 行高

`render_status_slot`：

- `width == 0` → 0 行。
- busy → Loader 两行（前导空行 + spinner）；cue 拼在 spinner 行。
- idle → 两行（前导空行 + cue 行或空白）。

固定区高度不随 busy/idle 切换变化。
