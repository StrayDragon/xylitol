# language: zh-CN
# migrated from tests/features/compaction.feature
# BDD 接线（tests/bdd.rs）：need-compact / no-compact（= should-compact true/false）/
# retain-recent / write-entry / branch-summary
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

  @req:c1
  场景: estimate-uses-priority
    假如 存在可信 XyUsage 锚点
    当 调用上下文估计
    那么 优先采用 Api 语义且仍返回统一估计结构

  @req:c1
  场景: estimate-fallback-chain
    假如 无 XyUsage 且 LocalTokenizer 可用
    当 调用上下文估计
    那么 采用 LocalTokenizer 而非静默当作 Api

  @req:c3
  场景: summarize
    假如 会话有 50 轮
    当 调用 compact
    那么 前 40 轮被摘要为一个 CompactionEntry

  @req:c4
  场景: persist
    假如 compaction 完成
    当 加载会话
    那么 存在含 summary 与切点的 CompactionEntry

  @req:c5
  场景: branch
    假如 用户导航到较早分支点
    当 生成分支摘要
    那么 摘要条目桥接上下文缺口

  @req:c7
  场景: generate-summary
    假如 会话有 30 轮 user+assistant 含文件编辑
    当 调用 generate_summary
    那么 响应含 Goal、Progress、Next Steps 节及具体文件路径

  @req:c8
  场景: find-cut
    假如 会话 50 条共 80000 tokens 且 keepRecent=20000
    当 调用 find_cut_point
    那么 切点索引大致保留最后 20000 tokens 上下文

  @req:c9
  场景: iterative
    假如 先前 CompactionEntry 含 summary，新消息已累积
    当 以 previousSummary 调用 generate_summary
    那么 结果保留先前 Done 项并添加新项

  @req:c10
  场景: files
    假如 消息含工具调用：read a.txt、write b.rs、edit c.py
    当 调用 compact_session
    那么 CompactionEntry summary 以 <read-files>a.txt</read-files> 与 <modified-files>b.rs c.py</modified-files> 结尾

  @req:c11
  场景: entry
    假如 compact_session 完成并加载会话
    当 CompactionEntry 存在
    那么 summary 非空、firstKeptEntryId 有效、tokensBefore 为正、details 含文件列表

  @req:c12
  场景: agent
    假如 agent 会话消息超阈值
    当 调用 compact_current_session
    那么 CompactionEntry 写入会话，会话状态已重载

  @req:c13
  场景: split-by-responsibility
    假如 compaction 公共 API 已就绪
    当 分别调用 should_compact、find_cut_point 与 compact_session
    那么 各 API 独立成功且返回预期结构

  @req:c15
  场景: provenance-available
    假如 完成一次启发式降级估计
    当 检查估计结果
    那么 带有 Heuristic 来源标注且无重复 XyUsage 定义

  @req:c2
  @req:c16
  场景: threshold-shares-footer-estimate
    假如 会话叶上存在可信 Api usage 锚点且 footer 同源估计可用
    当 执行 auto-compact 阈值判断
    那么 所用 token 数字与同源估计一致且 MUST NOT 另算独立 len/4 总和
