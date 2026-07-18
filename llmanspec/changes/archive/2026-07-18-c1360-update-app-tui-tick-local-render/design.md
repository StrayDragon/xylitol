# Design: c1360 Tick 局部刷新

## Host

- `paint_dirty: bool`：`append_bash_chunk` 置位；其它已同步 `request_render` 的路径不必经此标志。
- `Tick`：`anim = idle_tick()`；若 `anim || paint_dirty` 则清标志并 `request_render(false)`，否则跳过。

## UiRoot 上区缓存

- 缓存键：`width` + 内容世代（`upper_gen`，在 `apply_ui_model` / fold / theme / loaded_resources 变更时 +1）。
- `render`：若 `cached_width == width && cached_gen == upper_gen`，复用 `upper_lines`；否则重算 loaded + scrollback + queue 并写回缓存。
- status / editor / footer **每帧**仍渲染（spinner / 输入 / footer 可变）。

## 非目标

- 不改差异渲染引擎；不裁 `previous_lines`（c1370）。
