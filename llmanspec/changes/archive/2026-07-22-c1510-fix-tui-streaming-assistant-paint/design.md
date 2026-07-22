# Design: c1510 streaming assistant paint

## Boundaries

| 路径 | 本 change |
|---|---|
| `UiEntry::Assistant`（已提交） | 仍走 `ScrollbackPaintCache`（c1500）；不改 |
| `streaming_assistant` tail | **增量** Markdown paint |
| tool/bash/write/diff + Ctrl+O | **零改动**（`render_expandable_output` / `tools_output_expanded`） |

## Stable prefix

对当前 `streaming_assistant` 文本（不含尾部 `…`）：

1. 找「稳定前缀」终点：最后一个不在未闭合 ` ``` ` fence 内的 `\n\n`（段落边界）；若无则稳定前缀为空。
2. 缓存 `(width, stable_prefix_text) → prefix_lines`。
3. 每帧：若 width 不变且 `text` 仍以缓存的 `stable_prefix_text` 开头，则 `prefix_lines` + 仅对 `text[stable_len..] + "…"` 做 `Markdown::render`；否则全量重绘并刷新缓存。
4. 稳定前缀变长时：把新增稳定段渲染一次并 append 进 `prefix_lines`，再画新后缀。

## Correctness

- 可见行 MUST 与「对该帧全文 `Markdown::new(text+"…")`」在同 width/theme 下一致（测试比对或抽检）。
- fold/width/theme 变化 MUST 失效流式缓存（与 entry cache 相同）。

## Obs / harness

- `streaming_assistant_full_parses`（或等价）计数：连续 append delta 时不得每 delta +1 全量解析（有稳定段后应复用）。
