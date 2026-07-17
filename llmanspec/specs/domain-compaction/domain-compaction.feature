# language: zh-CN
# migrated from tests/features/compaction.feature
功能: domain-compaction
  背景:
    假定 有一个临时工作目录
    并且 配置了上下文窗口为 100000 的模型

  场景: need-compact
    假定 会话消息估算使用 90000 个 token
    并且 压缩阈值为 0.8
    当 调用 shouldCompact
    那么 返回 true

  场景: no-compact
    假定 会话消息估算使用 50000 个 token
    并且 压缩阈值为 0.8
    当 调用 shouldCompact
    那么 返回 false

  场景: retain-recent
    假定 会话有 50 个轮次
    当 触发压缩保留最近 10 轮
    那么 前 40 轮被总结为一个 CompactionEntry
    并且 会话中剩余 11 条记录（概要 + 10 轮）

  场景: write-entry
    假定 会话正在活跃使用
    当 压缩完成
    那么 会话 JSONL 包含 CompactionEntry
    并且 CompactionEntry 包含 summary 字段
    并且 CompactionEntry 包含 firstKeptEntryId 字段
    并且 CompactionEntry 包含 tokensBefore 字段

  场景: branch-summary
    假定 用户在树中导航到分支点
    当 生成分支摘要
    那么 摘要描述了被跳过的上下文
    并且 当前上下文是连贯的
