# Design: c1505 entry-level scrollback viewport slice

## Problem

`render_scrollback` 每帧：

1. 遍历**全部** `UiModel.entries`
2. cache hit 仍把该 entry 的全部行 `extend` 进输出 `Vec<String>`
3. 上层差分引擎再对整份 upper 做视口写屏优化——**不省**本函数的 CPU / 分配

c1500 解决的是「重复 Markdown paint」；本 change 解决「屏外历史仍进 flatten」。

## Approach A（选定）

```text
entries[0..N]  →  per-entry line counts (from ScrollbackPaintCache)
total_lines    →  sum(+ spacers)
follow-bottom  →  start_line = max(0, total - viewport_h - overscan)
               →  skip entries fully above window; partial first entry if needed
streaming_*    →  always append after historical window (live tail)
```

### Seam

| 层 | 职责 |
|---|---|
| `widgets/scrollback` | `render_scrollback(..., viewport: Option<ScrollbackViewport>)`；无 viewport = 今日全量（测试/兼容） |
| `layout` / host | 传入 terminal 可用 upper 高度 + scroll offset；follow-bottom 默认 |
| 差分引擎 | 不变；仍吃已切片的 `Vec<String>` |

### Non-goals

- Codex `insert_history` 分裂
- 按字符估高的虚拟列表
- 去掉宽度不变量

## Risks

| 风险 | 缓解 |
|---|---|
| 上滚空白 | overscan（≥1 屏）；滚动时补 paint 未缓存 entry |
| fold 改高度 | fold 变更 bump generation / 重算 offset |
| 流式空尾 | streaming tails 不参与「可跳过」集合 |

## Verification

1. Harness：大 N entries，follow-bottom 时 `render_scrollback` 输出行数 ≤ viewport+overscan+streaming（断言上界）
2. 可见内容 ath/PTY：首屏/贴底无缺字
3. `profile-suite` B：长种子下 scrollback flatten 样本占比相对基线下降
