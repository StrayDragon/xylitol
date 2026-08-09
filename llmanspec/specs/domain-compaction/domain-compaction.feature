# language: zh-CN
# migrated from tests/features/compaction.feature
# BDD 接线（tests/bdd.rs）：need-compact / no-compact / disabled-no-compact
# （should_compact + CompactionSettings.reserveTokens）/
# retain-recent / write-entry / branch-summary
功能: domain-compaction
  背景:
    假定 有一个临时工作目录
    并且 配置了上下文窗口为 100000 的模型

  @req:c2
  场景: need-compact
    假定 会话消息估算使用 90000 个 token
    并且 compaction reserveTokens 为 16384
    并且 compaction enabled 为 true
    当 调用 shouldCompact
    那么 返回 true

  @req:c2
  场景: no-compact
    假定 会话消息估算使用 50000 个 token
    并且 compaction reserveTokens 为 16384
    并且 compaction enabled 为 true
    当 调用 shouldCompact
    那么 返回 false

  @req:c2
  场景: disabled-no-compact
    假定 会话消息估算使用 90000 个 token
    并且 compaction reserveTokens 为 16384
    并且 compaction enabled 为 false
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

  @req:c8
  场景: cut-assistant
    假如 会话在 keep 预算内最近合法切点落在 assistant 消息
    当 调用 find_cut_point
    那么 切点落在该 assistant 且 is_split_turn 为 true 或 false 依是否 mid-turn 而定

  @req:c8
  场景: never-tool-result
    假如 会话含 toolResult 条目
    当 调用 find_cut_point
    那么 first_kept 永不落在 toolResult 索引

  @req:c8
  场景: keep-budget
    假如 keepRecent tokens 预算给定且存在多个合法切点
    当 调用 find_cut_point
    那么 保留侧上下文约等于 keepRecent 预算（最近合法切点）

  @req:c19
  场景: split-dual-summary
    假如 find_cut_point 返回 is_split_turn=true 且 turn_start 与 first_kept 之间有可摘要内容
    当 执行 split-turn compact_session
    那么 CompactionEntry.summary 含 Turn Context (split turn) 合并标记且 turn-prefix 已被摘要

  @req:c20
  场景: tokens-before
    假如 compact_session 完成
    当 读取 CompactionEntry.tokensBefore
    那么 该值来自压缩前会话上下文同源估计而非仅 boundary len/4 累加

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
  场景: reserve-trigger-shares-footer-estimate
    假如 会话叶上存在可信 Api usage 锚点且 footer 同源估计可用
    当 执行 auto-compact reserve 触发判断
    那么 所用 token 数字与同源估计一致且 MUST NOT 另算独立 len/4 总和
    并且 触发比较式为占用大于窗口减 reserveTokens

  @req:c26
  场景: turn-settlement-once-shared
    假如 单次 turn 收尾且未实际执行 compaction
    当 compact 预检与 footer 刷新完成
    那么 二者消费同一 settlement generation 且 MUST NOT 同秒重复独立 estimate 打点

  @req:c17
  @req:c2
  场景: auto-over-threshold
    假如 compaction enabled 为 true
    并且 同源估计已超过 window 减 reserveTokens
    并且 非 abort 的 assistant 回合刚落定
    当 执行 turn 后 threshold auto 检查
    那么 发生 compaction 且 CompactionStart reason 含 threshold

  @req:c17
  @req:c2
  场景: auto-under-threshold
    假如 compaction enabled 为 true
    并且 同源估计未超过 window 减 reserveTokens
    并且 非 abort 的 assistant 回合刚落定
    当 执行 turn 后 threshold auto 检查
    那么 不发生 compaction

  @req:c17
  @req:c2
  场景: auto-disabled-no-compact
    假如 compaction enabled 为 false
    并且 同源估计远超窗口
    并且 非 abort 的 assistant 回合刚落定
    当 执行 turn 后 threshold auto 检查
    那么 不发生 compaction

  @req:c17
  场景: manual-force-bypasses-reserve
    假如 用量未超 reserve 闸但 leaf 分支上有可摘要历史（按 pi 同构切点计量超出 keepRecent）
    当 调用 Driver 或 slash force compact
    那么 仍执行 compaction 或仅在末条已是 CompactionEntry 时返回 Already compacted
    并且 MUST NOT 经 maybe_auto_compact 闸
    并且 MUST NOT 因切点 JSON 低估把仍有可摘要历史误报为 Nothing to compact (no summarizable history beyond keep window) 或 session too small

  @req:c17
  @req:c25
  场景: compact-uses-leaf-branch-path
    假如 会话文件序含旁支 sibling 且当前 leaf 在右支
    当 执行 prepare 或 force compact
    那么 切点与摘要范围仅含 leaf 分支条目且不含左支 sibling

  @req:c18
  场景: stale-guard-after-compaction
    假如 刚写入 CompactionEntry
    并且 仅有压缩前 assistant usage 可用
    当 立即再执行 threshold auto 检查
    那么 MUST NOT 用压缩前 usage 再触发 compaction

  @req:c21
  @req:c22
  @req:c23
  场景: overflow-retry-ok
    假如 sameModel 的 assistant 被判定为 context overflow 且 stop_reason 非 stop
    并且 compaction enabled 且尚未做过 overflow recovery
    当 执行 turn 后 overflow 检查
    那么 发生 compaction 且 CompactionStart reason 含 overflow
    并且 工作上下文摘掉错误 assistant 后续跑模型且重试成功

  @req:c22
  场景: overflow-once
    假如 本回合已完成一次 overflow compact-and-retry
    并且 再次出现 sameModel overflow
    当 执行 turn 后 overflow 检查
    那么 MUST NOT 再次 compact 或无限重试
    并且 CompactionEnd 含固定失败说明文案且 will_retry 为 false

  @req:c22
  场景: wrong-model
    假如 assistant 的 provider 或 model 与当前模型不同且该 assistant 为 overflow
    当 执行 turn 后 overflow 检查
    那么 MUST NOT 因该旧 overflow 触发 recovery

  @req:c23
  @req:c17
  场景: reason-overflow
    假如 overflow Case1 触发 auto-compact
    当 观察 CompactionStart 与 CompactionEnd
    那么 reason 可区分为 overflow 且与 threshold 或 manual 不同

  @req:c24
  场景: bare-force
    假如 会话可 compact 且无 instructions
    当 执行手动 force compact
    那么 发生 compaction 且送入摘要模型的 prompt MUST NOT 含 Additional focus

  @req:c24
  场景: with-text
    假如 会话可 compact 且 instructions 为非空文本
    当 执行手动 force compact 并传入该文本
    那么 history 摘要 prompt MUST 含 Additional focus 与该文本

  @req:c24
  场景: whitespace
    假如 instructions 仅空白或 None
    当 执行手动 force compact
    那么 行为等同无 instructions（prompt 无 Additional focus）

  @req:c24
  场景: auto-clean
    假如 threshold 或 overflow auto-compact 触发
    当 观察摘要模型输入
    那么 MUST NOT 含 Additional focus 且 MUST NOT 复用上一次 manual instructions
