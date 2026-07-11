# Design — c540-update-package-tui-diff-edges

## 范围

只打磨包内 Diff 边角；算法与主题闭包不变。

## 边角清单

| 项 | 期望 |
|---|---|
| 窄宽 + CJK | `visible_width` 折行/截断，不按字节劈开 |
| SBS 空半栏 | 无配对侧不伪造行号 |
| 回归 | edit-format 与 SBS 关键至少各一 snapshot 或等价 assert |

## 非目标

- 新 Diff 布局方言；产品 tint 策略（仍见 expandable / design）
