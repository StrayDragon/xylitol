# Compaction status（草稿）

后置：上下文压缩 / 重试状态（c493）。

## 意向

- 走 status 一行（Retry… / Compacting…）；idle 不占行。
- 详情进日志或可展开一行，勿底栏增高。
