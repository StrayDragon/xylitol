---
depends_on:
  - c2858-add-package-tui-pointer-hit-pipeline
---

# 右侧一列滚动槽（跳顶 / 跳底）

## Why

ApplicationOwned 视口自管滚动之后，长 transcript 只有滚轮 / 拖选贴边自动滚，没有「我在全文哪一段」的位置暗示，也不能一键到顶/到底。

这不是左右边栏（c2875 park、DESIGN **无双栏**）。只要 **transcript 视口右缘 1 列 gutter**：

```
^     ← 点这里：滚到顶
|
|
v     ← 点这里：滚到底
```

点按必须挂在 c2858 的命中管线上：右缘是 capturing 区，Left Down 不得启 transcript 拖选。`^` / `v` 钉在**当前可见页**的顶/底行，每帧用 live `scroll_top` 重登 HitRegion（不要冻结 content 行号，否则一滚按钮就跟着内容跑走）。

## What Changes（ff 时落地）

- 包层可选：ApplicationOwned transcript 右缘 1 列滚动槽（默认关，host opt-in）。
- 绘出：顶 `^`、中轨 `|`（或 thumb 以后加）、底 `v`。不引入 ratatui `Constraint` / 双栏 layout。
- 命中：capturing `HitRegion`；点 `^` → `scroll_top = 0`；点 `v` → `scroll_to_end`。中轨先只吞拖选，不强制分页（ff 再定要不要 page / 拖 thumb）。
- 产品接线另票或同票末刀：仅 ApplicationOwned；Inline 仍走终端 scrollback。
- **不**修订 DESIGN「无双栏」；**不**做 Workspace/Plan/Files 侧栏。

## 非目标（草案）

- 不实施（本目录仅 proposal）。
- 不抄 IDE 宽滚动条 / overlay 抽屉 / 宽度动画。
- 不把滚动槽做成第二条 transcript。

## Capabilities（ff 时）

- 新或扩 `package-tui-pointer` / 视口相关包 capability（包 generic 先于产品）
- 产品若接线：`app-tui` 固定区词汇另开，勿叫边栏 / 待办栏 / 槽

## Impact

- transcript 有效宽 −1 列（wrap 缓存、Diff SBS 阈值需在 ff 时量一次）。
- 依赖 c2858：无命中管线则右缘点击仍会拖选文本。

## Open Questions（ff 时拍板）

1. 中轨要不要显示 thumb（视口/全文比）？草案默认：先 `^ | v`，thumb 可后加。
2. 点中轨 = 忽略，还是 page up/down？
3. 窄于某列数时是否自动隐藏 gutter？

## Further Notes

- 落实路径：用户指定 **`llman-sdd-ff`** 完整化（propose 等价：规划 → 绑定分支 → 落地 specs），再 apply。
- 对照 park 的边栏票：[c2875](../c2875-add-tui-sidebar-panes/proposal.md)（占用列 / 抽屉，默认不实施）。
- 命中几何真源：c2858 `HitRegion` = content 行 + 列区间；screen 格 → `content_row = scroll_top + screen_row`。
