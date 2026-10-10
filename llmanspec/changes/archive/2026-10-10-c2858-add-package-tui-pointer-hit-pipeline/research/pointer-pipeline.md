# 指针管线：xylitol 现状 vs ratatui vs 前置缝

调研时点：2026-10-10。

## xylitol 今日序

```
1003 Moved ──产品 map_crossterm_item 丢──► 到不了 dispatch_event
Down ──清 editor 选区──► hit_priority? ──yes──► swallow / 不启拖 ──Rerender──► host fold_dirty ► stale flatten
                     └──no──► start drag
Drag / Up ──仅 dragging；Up 禁止 hit_priority
Wheel ── ingest ── reproject（不 flatten）
AO 鼠标 在 overlay 之前，点「盖在 transcript 上的东西」仍启选区
```

关键符号：

- `TUI::dispatch_event` `packages/xylitol-tui/src/tui/mod.rs` ~842
- `SelectionController::handle_mouse` `selection.rs` ~219；`HitPriorityFn` 只 Down
- 产品丢 Moved：`src/app/tui/mod.rs` ~580
- `FoldHitTable` 在产品 `src/app/tui/widgets/fold_hit.rs`（content 坐标）
- 选区 tint：`apply_highlight` 在 `paint_visible` 改可见切片——hover 应学这个，不要 `ao_components_stale`

## ratatui（可借分工，不借类型）

- `Widget::render(Rect, &mut Buffer)` 只有画；**零** `MouseEvent`。
- App：`EnableMouseCapture` + 上一帧 `Rect.contains(column, row)` 改 state。
- `Tabs`：水平排 title，**不返回每 tab Rect**；要点击得自己量宽。
- `Layout::split` + `Constraint` = 2D 栏。xylitol `dock_rows` 是底 N 行全宽；DESIGN 无双栏。
- **禁止**依赖 ratatui / 抄 Buffer·Span·Constraint 当默认布局（D12）。

## ratatui-markdown

- 折叠树是键盘 + hybrid scroll engaged，**不点**。
- mermaid → `Vec<Line>` box-drawing；**SySL-1.0**，禁止进 xylitol。
- hybrid free/engaged 是键盘焦点捕获，不是 opencode Down-enter。

## 为何必须先做本票

| 后来者 | 卡点 |
|---|---|
| c2860 整块 click | Up 禁止 hit；扩 Down 矩形会偷拖选 |
| c2860 hover | Moved 双丢；无廉价行 overlay |
| c2875 overlay 抽屉 | 选区先于 overlay |
| c2880 可点 tab | dock/chrome 无通用 hit |
| mermaid pan（c2890 之后） | Drag 一律进 SelectionController |

c2865 打字机、c2890 ASCII 渲染、c2880 键盘 Tab **不**依赖本票。
