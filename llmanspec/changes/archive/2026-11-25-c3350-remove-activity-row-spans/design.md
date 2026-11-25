# Design: c3350-remove-activity-row-spans

## 目标

折叠状态不再维护一份没有读者的行区间表。

## 两种形状

| | 形状 A | 形状 B |
|---|---|---|
| 改动 | 删除 `SegmentRowSpans`、字段、`insert` 和 `clear` | 给 `row_spans` 找一个读者，让绘制去用它 |

## 选择

用形状 A。

全仓没有读者。折叠级别、命中表和展开状态有读者，保留。

## 完成时的读法

`ActivityFoldState::row_spans` 从 `deslop-please` 报告里消失。`ActivityFoldState::settings` 的泄漏留在基线里。
