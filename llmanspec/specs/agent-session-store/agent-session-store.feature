# language: zh-CN
# capability: agent-session-store
# purpose: 会话持久化 — 基于 JSONL 文件的会话存储，含树操作、compaction、fork 与 CWD 校验。
# scope: infra 层 session 持久化

功能: agent-session-store

  @req:r31 @human
  场景: 快照操作
    - System MUST 支持对不可变会话快照执行 snapshot、restore、spawn、list、prune、diff、merge 操作。

  @req:r41 @human
  场景: compaction
    - 当 CompactionSettings.enabled 且上下文占用超过 window−reserve（c1630）时，System MUST 在 agent/capabilities 回合落定后自动 compact（c1640）；MUST NOT 仅依赖百分比闸或仅 TUI 轮询。

  @req:s1 @human
  场景: JSONL 存储
    - SessionManager MUST 将会话持久化为 JSONL 文件（每行一个带版本标签的 JSON 对象），位于 ~/.xylitol/sessions/。

  @req:s2 @human
  场景: 条目类型
    - SessionManager MUST 支持条目类型（JSONL type 判别为 camelCase）：message、compaction、branchSummary、modelChange、thinkingLevelChange、custom、customMessage、label、sessionInfo；bang-bash 不作为顶层 type，而走 message+role=bashExecution。MUST NOT 将 snake_case type 标签（如 branch_summary、bash_execution）视为合法 SSOT。

  @req:s3 @human
  场景: CRUD
    - SessionManager MUST 支持 create、append、load、list、exists 操作。

  @req:s4 @human
  场景: 禁止旧 AgentPart 迁移
    - Session JSONL 的 message.content AgentPart 形态以 domain-message dm1 为唯一真源；系统 MUST NOT 迁移或静默解读 c646 之前的 untagged/裸字符串 content。读到此类旧 content 时 MUST 按 agent-session as46 拒绝/跳过策略处理。文件级其它版本字段若存在 MAY 保留，但 MUST NOT 借此复活旧 AgentPart 语义。

  @req:s5 @human
  场景: 会话树
    - SessionManager MUST 维护 parent/child 会话链接：fork 创建子分支，branchSummary 引用父条目。

  @req:s6 @human
  场景: 分支摘要
    - System MUST 支持 generateBranchSummary(parentEntries)，产出汇总切点前条目的 CompactionEntry。

  @req:s7 @human
  场景: 写入安全
    - SessionManager 写会话文件 MUST 满足崩溃原子性：进程任意时点终止后，盘上会话文件要么保持旧完整内容、要么呈现新完整内容，MUST NOT 出现截断或半行混合状态。系统以单进程单写者为并发假设（同一会话至多一个进程写入），MUST NOT 声称提供跨进程文件锁互斥。

  @req:s9 @human
  场景: 会话 fork
    - SessionManager MUST 支持 fork(parent_id, child_id, at_entry_id)，将父条目复制到切点至新子会话文件并 append branch_summary 条目。

  @req:s10 @human
  场景: 分支摘要实现
    - fork 时 System MUST 生成描述被跳过条目的分支摘要：条目数、条目类型、最后 user 消息及工具调用中的 notable actions。

  @req:s11 @human
  场景: agent fork
    - System MUST 支持 fork_session(at_entry_id)：经 SessionManager::fork() 创建新子会话并返回子 session id。

  @req:s16 @human
  场景: 会话 CWD 校验
    - 从磁盘加载会话时 SessionManager MUST 校验存储 CWD 存在且可访问；不可用时返回可操作错误信息。

  @req:s18 @human
  场景: session-entry-camelcase-v5
    - Session persisted JSONL MUST 以 camelCase 为唯一磁盘格式 SSOT（JS/TS favor；非 pi snake entry type）：外壳字段含 parentId、parentSession、firstKeptEntryId 等；SessionEntry type 判别为 message、compaction、branchSummary、modelChange、thinkingLevelChange、custom、customMessage、label、sessionInfo。新写入的 bang-bash MUST 使用 type=message 且 message.role=bashExecution，MUST NOT 再写出顶层 type=bashExecution 或 bash_execution。新写入的 header.version MUST 等于 SESSION_VERSION（6）。MUST NOT 再写出 parent_id 或 version≤5 作为新会话真源。MUST NOT 提供 serde alias、静默 v3/v4/v5 migrate、或将旧顶层 bash 提升为合法上下文。

  @req:s19 @human
  场景: tool-result-tool-call-id
    - role=toolResult 的 AgentMessage MUST 以 toolCallId 键持久化工具调用关联 id（对齐 pi）；MUST NOT 写出 toolUseId。content 内 MUST NOT 再嵌入 type=toolResult 的 AgentPart（工具结果只走独立 message 行）。

  @req:s20 @human
  场景: session-load-skip-warn
    - load（及同源逐行解析）遇到无法按最新 SSOT 解析的行（坏 JSON、未知 type、非 SSOT snake type 如 bash_execution、旧 untagged content 导致无法投影）时 MUST 跳过该行并记录可观测 warn；同一 load/list 操作内明文 warn MUST 至多 3 条，超出后 MUST 以单条省略标记（如 ...）收敛，MUST NOT 刷屏。header.version 不等于 SESSION_VERSION（6）时 MUST 拒绝将该文件视为合法最新会话（返回可操作错误），MUST NOT 静默 migrate 后当成功。TUI resume、print/CLI --session 与 SessionManager MUST 共用此策略。

  @req:s21 @human
  场景: list-sessions-resilient
    - list_sessions（或等价枚举）遇到单个会话文件不可读/非最新/解析失败时 MUST 跳过该文件（或给出占位诊断）并继续枚举其余会话，MUST NOT 因单文件失败而使整表 list/resume 面板失败。

  @req:ex1 @human
  场景: 导出 HTML
    - System MUST 支持将会话导出为 HTML 文件。

  @req:ex2 @human
  场景: 导出 JSONL
    - System MUST 支持将会话导出为 JSONL 文件。

  @req:ex3 @human
  场景: 导入 JSONL
    - System MUST 支持从 JSONL 文件导入会话为新会话。

  @req:ex4 @human
  场景: HTML 工具渲染
    - HTML 导出 MUST 将 tool 与 assistant 内容渲染为可读块。

  @req:ex5 @human
  场景: 分享指引
    - 未配置 token 时调用分享，System MUST 返回配置指引。

  @req:sc1 @human
  场景: CWD 校验
    - 恢复会话时 System MUST 校验会话头存储的 cwd 在磁盘可用；不可用时 MUST 尝试 fallback cwd，其一可用即继续。

  @req:sc2 @human
  场景: CWD 错误
    - 两者均不可用时 System MUST 返回可操作校验错误，消息 MUST 同时携带存储 cwd 与 fallback cwd。

  @req:sc3 @human
  场景: CWD 呈现
    - 面向用户的 CWD 错误信息 MUST 即该可操作校验错误文本（引导用户落到 fallback cwd）。

  @req:sc4 @human
  场景: CWD 集成
    - CLI 与 RPC 模式 MUST 在恢复会话前完成同一校验。

  @req:sp1 @human
  场景: SessionStore 端口实现
    - infra 层的 SessionManager MUST 实现 protocol 的 XySessionStore 端口（append、load、load_context、exists），仅覆盖 agent 循环与 compaction 所需；完整会话面（fork、navigate、export）MAY 留在具体结构体供组合根直接使用。

  @req:sp6 @human
  场景: 日志访问
    - 会话 store MUST 暴露 read_recent(session_id, limit) -> Vec<Event> 供 server journal；journal 容量 MUST 可配置（默认 10000）。

  @req:sp3 @human
  场景: ExportIo 实现
    - System MUST 提供导出 I/O 端口实现（StdExportIo 或等价，tokio::fs 读写）；组合根 MUST 将导出协作器经该端口注入。

  @req:s22 @human
  场景: 磁盘格式时间戳
    - Session 磁盘格式时间戳（header.timestamp 与条目壳 timestamp）MUST 为 u64 unix 毫秒数，与 message 内 timestamp 同基准；MUST NOT 写出 RFC3339 字符串作为磁盘格式时间戳。展示面需要人类可读时间时 MUST 在展示边格式化。

  @req:s12 @human
  场景: 延迟持久化至 assistant
    - Persisted SessionManager（非 in_memory backend）MUST 对齐 pi：首条 assistant 消息写入前，会话条目可仅保留在内存；首条 assistant append 时 MUST 创建磁盘 jsonl（若尚不存在）并刷出此前挂起条目（含 header 与先前 user 等）；其后每次 append MUST 落盘。InMemory backend MUST NOT 写文件。MUST NOT 在仅有 header、用户又放弃对话时强制留下无消息垃圾文件作为唯一策略（允许 create 仍写 header，但 deferred 刷出路径 MUST 存在并可测）。
  @executable @req:s1
  场景: create-load
    假如 会话存储目录已初始化
    当 创建一个新会话 "test-session"
    并且 向会话追加一条消息 "用户问好"
    并且 加载会话 "test-session"
    那么 会话包含 1 条记录
    并且 记录类型为 "message"

  @executable @req:s3
  场景: list-sessions
    假如 存在会话 "session-a"
    并且 存在会话 "session-b"
    当 列出所有会话
    那么 结果包含 "session-a"
    并且 结果包含 "session-b"

  @executable @req:s5
  场景: fork
    假如 存在会话 "parent" 包含 10 条记录
    当 在记录 5 处分叉创建会话 "child"
    那么 会话 "child" 包含 5 条记录
    并且 会话 "child" 包含一个 branch_summary 记录

  @executable @req:s5
  场景: tree-nav
    假如 存在会话树: "root" → "branch-a" → "branch-b"
    当 导航到 "branch-b"
    那么 上下文包含 branch-a 和 branch-b 的摘要

  @executable @req:s2
  场景: model-change
    假如 存在会话 "model-test"
    当 将会话模型从 "gpt-4o" 切换为 "claude-sonnet"
    那么 会话包含 model_change 记录
    并且 model_change 记录显示 provider 为 "anthropic"

  @executable @req:s2
  场景: thinking-change
    假如 存在会话 "think-test"
    当 切换思考级别为 "high"
    那么 会话包含 thinking_level_change 记录

  @executable @req:s1
  场景: jsonl-format
    假如 存在会话 "format-test"
    当 向会话追加 3 条不同类型的记录
    那么 JSONL 文件每行是一个完整的 JSON 对象
    并且 第一行包含 version 字段

  @executable @req:s2
  场景: label-set
    假如 存在会话 "label-test"
    当 向会话追加一条消息 "标记我"
    并且 为最后一条记录设置标签 "重要"
    那么 该记录的标签为 "重要"

  @executable @req:s2
  场景: label-clear
    假如 存在会话 "label-clear-test"
    当 向会话追加一条消息 "标签测试"
    并且 为最后一条记录设置标签 "重要"
    并且 清除该记录的标签
    那么 该记录没有标签

  @req:s12 @executable
  场景: delayed-header-flush-on-first-append
    假如 会话存储目录已初始化
    当 创建一个新会话 "lazy"
    那么 会话 "lazy" 的磁盘 JSONL 尚不存在
    当 向会话追加用户消息 "hello"
    那么 会话 "lazy" 的磁盘 JSONL 已存在且包含 "hello"

  @req:s9 @executable
  场景: fork-copies-cutoff-and-summary
    假如 存在会话 "parent" 包含 6 条记录
    当 在记录 3 处分叉创建会话 "child"
    那么 会话 "child" 包含一个 branch_summary 记录
    并且 会话 "child" 包含文本 "message 2"
    并且 会话 "child" 不含文本 "message 5"

  @req:s10 @executable
  场景: branch-summary-content
    假如 存在会话 "parent" 包含 6 条记录
    当 在记录 4 处分叉创建会话 "child"
    那么 会话 "child" 的 branch_summary 摘要包含 "message 4"
    并且 会话 "child" 不含文本 "message 5"

  @req:s11 @executable
  场景: fork-session-returns-child-id
    假如 存在会话 "parent" 包含 4 条记录
    当 在记录 2 处分叉创建会话 "kid"
    那么 会话 "kid" 包含文本 "message 1"
    并且 会话 "kid" 包含一个 branch_summary 记录

  @req:s21 @executable
  场景: list-resilient-to-corrupt-file
    假如 存在会话 "good" 包含 2 条记录
    当 目录中植入损坏的会话文件 broken.jsonl
    当 列出所有会话
    那么 结果包含 "good"

  @req:s22 @executable
  场景: timestamps-u64-ms
    假如 存在会话 "ts1" 包含 2 条记录
    当 加载会话 "ts1"
    那么 会话 "ts1" 的 JSONL 时间戳均为 u64 毫秒
