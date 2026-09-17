# language: zh-CN
# capability: agent-session-store
# purpose: 会话持久化 — 基于 JSONL 文件的会话存储，含树操作、compaction、fork 与 CWD 校验。
# scope: src/infra/session/

功能: agent-session-store

  @req:r31 @human
  场景: 快照操作
    - System MUST 支持对不可变会话快照执行 snapshot、restore、spawn、list、prune、diff、merge 操作。

  @req:r41 @human
  场景: compaction
    - 当 CompactionSettings.enabled 且上下文占用超过 window−reserve（c1630）时，System MUST 在 agent/capabilities 回合落定后自动 compact（c1640）；MUST NOT 仅依赖百分比闸或仅 TUI 轮询。

  @req:r1090 @human
  场景: JSONL 存储
    - SessionManager MUST 将会话持久化为 v7 会话目录（位于 ~/.xylitol/sessions/），目录 MUST 由 manifest 提交指针、一个 active JSONL 段与零个或多个 sealed cold JSONL 段组成；每行仍是一个带版本标签的 JSON 对象，manifest MUST 是恢复时的 SSOT。

  @req:r1097 @human
  场景: 条目类型
    - SessionManager MUST 支持条目类型（JSONL type 判别为 camelCase）：message、compaction、branchSummary、modelChange、thinkingLevelChange、custom、customMessage、label、sessionInfo；bang-bash 不作为顶层 type，而走 message+role=bashExecution。MUST NOT 将 snake_case type 标签（如 branch_summary、bash_execution）视为合法 SSOT。

  @req:r1105 @human
  场景: CRUD
    - SessionManager MUST 支持 create、append、load、list、exists 操作。

  @req:r1106 @human
  场景: 禁止旧 AgentPart 迁移
    - Session JSONL 的 message.content AgentPart 形态以 domain-message dm1 为唯一真源；系统 MUST NOT 迁移或静默解读 c646 之前的 untagged/裸字符串 content。读到此类旧 content 时 MUST 按 agent-session as46 拒绝/跳过策略处理。文件级其它版本字段若存在 MAY 保留，但 MUST NOT 借此复活旧 AgentPart 语义。

  @req:r1107 @human
  场景: 会话树
    - SessionManager MUST 维护 parent/child 会话链接：fork 创建子分支，branchSummary 引用父条目。

  @req:r1108 @human
  场景: 分支摘要
    - System MUST 支持 generateBranchSummary(parentEntries)，产出汇总切点前条目的 CompactionEntry。

  @req:r1109 @human
  场景: 写入安全
    - SessionManager 写 v7 会话 MUST 以 manifest 原子切换作为 seal 提交点：进程任意时点终止后，旧 manifest 仍 MUST 指向可恢复的旧段，或新 manifest MUST 只指向完整的新段；MUST NOT 将未提交的 orphan/临时段拼入会话。系统以单进程单写者为并发假设（同一会话至多一个进程写入），MUST NOT 声称提供跨进程文件锁互斥。

  @req:r1110 @human
  场景: 会话 fork
    - SessionManager MUST 支持 fork(parent_id, child_id, at_entry_id)，将父当前 leaf 路径（需要时按需读取 cold 段）复制到新子会话的 v7 active 段并 append branch_summary 条目；子会话 MUST NOT 保存对父 session 段文件的跨目录引用。

  @req:r1091 @human
  场景: 分支摘要实现
    - fork 时 System MUST 生成描述被跳过条目的分支摘要：条目数、条目类型、最后 user 消息及工具调用中的 notable actions。

  @req:r1092 @human
  场景: agent fork
    - System MUST 支持 fork_session(at_entry_id)：经 SessionManager::fork() 创建新子会话并返回子 session id。

  @req:r1101 @human
  场景: fork-header-cut-entry
    - fork 创建子会话时，子会话头 MUST 记录父会话 id，且 MUST 记录切点条目 id（fork 时所选条目，含 Before 切位时未拷入子会话的那条）；非 fork 创建的会话头 MUST NOT 写入切点字段。加载缺少该切点字段的旧会话文件时 MUST 视为无切点，MUST NOT 因此失败。

  @req:r1102 @human
  场景: v7 manifest 提交
    - v7 manifest MUST 只引用 session 目录内的完整段文件并记录 active 段与 sealed 段的逻辑顺序；compaction seal 成功后已引用的 cold 段 MUST NOT 再被改写，未被 manifest 引用的临时或 orphan 段 MUST NOT 影响 load、resume 或 list。

  @req:r1103 @human
  场景: 冷段按需恢复
    - resume、load_leaf_branch 与 session context 构造 MUST 先读取 manifest 与 active 段，仅在当前 leaf 的 parentId/分支或会话级配对需要时按需读取命中的 cold 段；MUST NOT 为普通 resume 无条件解析全部 cold 段。完整导出或 inspect 可显式读取完整逻辑条目流。

  @req:r1104 @human
  场景: v6 一次性迁移
    - 首次访问仅有 v6 单文件 `{id}.jsonl` 且无 v7 manifest 的会话时，System MUST 幂等地迁移为 v7 目录并在 manifest 提交成功后清理旧文件；迁移失败 MUST 保留旧文件并返回可操作错误；v5 及更早或未知版本 MUST 拒绝，MUST NOT 建立长期双读兼容路径。

  @req:r1094 @human
  场景: 会话 CWD 校验
    - 从磁盘加载会话时 SessionManager MUST 校验存储 CWD 存在且可访问；不可用时返回可操作错误信息。

  @req:r1095 @human
  场景: session-entry-camelcase-v7
    - Session persisted JSONL MUST 以 camelCase 为唯一磁盘格式 SSOT（JS/TS favor；非 pi snake entry type）：v7 段外壳字段含 parentId、parentSession、firstKeptEntryId 等；SessionEntry type 判别为 message、compaction、branchSummary、modelChange、thinkingLevelChange、custom、customMessage、label、sessionInfo。新写入的 bang-bash MUST 使用 type=message 且 message.role=bashExecution，MUST NOT 再写出顶层 type=bashExecution 或 bash_execution。新写入的 header.version MUST 等于 SESSION_VERSION（7），manifest 的 formatVersion MUST 同为 7。MUST NOT 再写出 parent_id 或 version≤5 作为新会话真源。v6 只允许经 s26 一次性迁移；MUST NOT 提供长期 serde alias 或旧顶层 bash 升格为合法上下文。

  @req:r1096 @human
  场景: tool-result-tool-call-id
    - role=toolResult 的 AgentMessage MUST 以 toolCallId 键持久化工具调用关联 id（对齐 pi）；MUST NOT 写出 toolUseId。content 内 MUST NOT 再嵌入 type=toolResult 的 AgentPart（工具结果只走独立 message 行）。

  @req:r1098 @human
  场景: session-load-skip-warn
    - load（及同源逐行解析）遇到无法按最新 SSOT 解析的行（坏 JSON、未知 type、非 SSOT snake type 如 bash_execution、旧 untagged content 导致无法投影）时 MUST 跳过该行并记录可观测 warn；同一 load/list 操作内明文 warn MUST 至多 3 条，超出后 MUST 以单条省略标记（如 ...）收敛，MUST NOT 刷屏。v7 manifest/header version 不等于 SESSION_VERSION（7）时 MUST 拒绝将其视为合法最新会话（返回可操作错误）；仅 v6 遗留单文件可先按 s26 完成迁移，MUST NOT 静默把 v5 及更早版本 migrate 后当成功。TUI resume、print/CLI --session 与 SessionManager MUST 共用此策略。

  @req:r1099 @human
  场景: list-sessions-resilient
    - list_sessions（或等价枚举）遇到单个 v7 manifest/段不可读、非最新或解析失败时 MUST 跳过该 session（或给出占位诊断）并继续枚举其余会话；发现 v6 遗留单文件时 MUST 按 s26 识别或迁移，MUST NOT 因单 session 失败而使整表 list/resume 面板失败。

  @req:r1085 @human
  场景: 导出 HTML
    - System MUST 支持将会话导出为 HTML 文件。

  @req:r1086 @human
  场景: 导出 JSONL
    - System MUST 支持将会话导出为 JSONL 文件。

  @req:r1087 @human
  场景: 导入 JSONL
    - System MUST 支持从 JSONL 文件导入会话为新会话。

  @req:r1088 @human
  场景: HTML 工具渲染
    - HTML 导出 MUST 将 tool 与 assistant 内容渲染为可读块。

  @req:r1089 @human
  场景: 分享指引
    - 未配置 token 时调用分享，System MUST 返回配置指引。

  @req:r1111 @human
  场景: CWD 校验
    - 恢复会话时 System MUST 校验会话头存储的 cwd 在磁盘可用；不可用时 MUST 尝试 fallback cwd，其一可用即继续。

  @req:r1112 @human
  场景: CWD 错误
    - 两者均不可用时 System MUST 返回可操作校验错误，消息 MUST 同时携带存储 cwd 与 fallback cwd。

  @req:r1113 @human
  场景: CWD 呈现
    - 面向用户的 CWD 错误信息 MUST 即该可操作校验错误文本（引导用户落到 fallback cwd）。

  @req:r1114 @human
  场景: CWD 集成
    - CLI 与 RPC 模式 MUST 在恢复会话前完成同一校验。

  @req:r1115 @human
  场景: SessionStore 端口实现
    - infra 层的 SessionManager MUST 实现 protocol 的 XySessionStore 端口（append、load、load_context、exists），仅覆盖 agent 循环与 compaction 所需；完整会话面（fork、navigate、export）MAY 留在具体结构体供组合根直接使用。

  @req:r1117 @human
  场景: 日志访问
    - 会话 store MUST 暴露 read_recent(session_id, limit) -> Vec<Event> 供 server journal；journal 容量 MUST 可配置（默认 10000）。

  @req:r1116 @human
  场景: ExportIo 实现
    - System MUST 提供导出 I/O 端口实现（StdExportIo 或等价，tokio::fs 读写）；组合根 MUST 将导出协作器经该端口注入。

  @req:r1100 @human
  场景: 磁盘格式时间戳
    - Session 磁盘格式时间戳（header.timestamp 与条目壳 timestamp）MUST 为 u64 unix 毫秒数，与 message 内 timestamp 同基准；MUST NOT 写出 RFC3339 字符串作为磁盘格式时间戳。展示面需要人类可读时间时 MUST 在展示边格式化。

  @req:r1093 @human
  场景: 延迟持久化至 assistant
    - Persisted SessionManager（非 in_memory backend）MUST 对齐 pi：首条 assistant 消息写入前，会话条目可仅保留在内存；首条 assistant append 时 MUST 创建磁盘 jsonl（若尚不存在）并刷出此前挂起条目（含 header 与先前 user 等）；其后每次 append MUST 落盘。InMemory backend MUST NOT 写文件。MUST NOT 在仅有 header、用户又放弃对话时强制留下无消息垃圾文件作为唯一策略（允许 create 仍写 header，但 deferred 刷出路径 MUST 存在并可测）。
  @executable @req:r1090
  场景: create-load
    假如 会话存储目录已初始化
    当 创建一个新会话 "test-session"
    并且 向会话追加一条消息 "用户问好"
    并且 加载会话 "test-session"
    那么 会话包含 1 条记录
    并且 记录类型为 "message"

  @executable @req:r1105
  场景: list-sessions
    假如 存在会话 "session-a"
    并且 存在会话 "session-b"
    当 列出所有会话
    那么 结果包含 "session-a"
    并且 结果包含 "session-b"

  @executable @req:r1107
  场景: fork
    假如 存在会话 "parent" 包含 10 条记录
    当 在记录 5 处分叉创建会话 "child"
    那么 会话 "child" 包含 5 条记录
    并且 会话 "child" 包含一个 branch_summary 记录

  @executable @req:r1107
  场景: tree-nav
    假如 存在会话树: "root" → "branch-a" → "branch-b"
    当 导航到 "branch-b"
    那么 上下文包含 branch-a 和 branch-b 的摘要

  @executable @req:r1097
  场景: model-change
    假如 存在会话 "model-test"
    当 将会话模型从 "gpt-4o" 切换为 "claude-sonnet"
    那么 会话包含 model_change 记录
    并且 model_change 记录显示 provider 为 "anthropic"

  @executable @req:r1097
  场景: thinking-change
    假如 存在会话 "think-test"
    当 切换思考级别为 "high"
    那么 会话包含 thinking_level_change 记录

  @executable @req:r1090
  场景: jsonl-format
    假如 存在会话 "format-test"
    当 向会话追加 3 条不同类型的记录
    那么 JSONL 文件每行是一个完整的 JSON 对象
    并且 第一行包含 version 字段

  @executable @req:r1097
  场景: label-set
    假如 存在会话 "label-test"
    当 向会话追加一条消息 "标记我"
    并且 为最后一条记录设置标签 "重要"
    那么 该记录的标签为 "重要"

  @executable @req:r1097
  场景: label-clear
    假如 存在会话 "label-clear-test"
    当 向会话追加一条消息 "标签测试"
    并且 为最后一条记录设置标签 "重要"
    并且 清除该记录的标签
    那么 该记录没有标签

  @req:r1093 @executable
  场景: delayed-header-flush-on-first-append
    假如 会话存储目录已初始化
    当 创建一个新会话 "lazy"
    那么 会话 "lazy" 的磁盘 JSONL 尚不存在
    当 向会话追加用户消息 "hello"
    那么 会话 "lazy" 的磁盘 JSONL 已存在且包含 "hello"

  @req:r1110 @executable
  场景: fork-copies-cutoff-and-summary
    假如 存在会话 "parent" 包含 6 条记录
    当 在记录 3 处分叉创建会话 "child"
    那么 会话 "child" 包含一个 branch_summary 记录
    并且 会话 "child" 包含文本 "message 2"
    并且 会话 "child" 不含文本 "message 5"

  @req:r1091 @executable
  场景: branch-summary-content
    假如 存在会话 "parent" 包含 6 条记录
    当 在记录 4 处分叉创建会话 "child"
    那么 会话 "child" 的 branch_summary 摘要包含 "message 4"
    并且 会话 "child" 不含文本 "message 5"

  @req:r1092 @executable
  场景: fork-session-returns-child-id
    假如 存在会话 "parent" 包含 4 条记录
    当 在记录 2 处分叉创建会话 "kid"
    那么 会话 "kid" 包含文本 "message 1"
    并且 会话 "kid" 包含一个 branch_summary 记录

  @req:r1099 @executable
  场景: list-resilient-to-corrupt-file
    假如 存在会话 "good" 包含 2 条记录
    当 目录中植入损坏的会话文件 broken.jsonl
    当 列出所有会话
    那么 结果包含 "good"

  @req:r1100 @executable
  场景: timestamps-u64-ms
    假如 存在会话 "ts1" 包含 2 条记录
    当 加载会话 "ts1"
    那么 会话 "ts1" 的 JSONL 时间戳均为 u64 毫秒

  @req:r1086 @executable
  场景: export-jsonl
    假如 会话存储目录已初始化
    当 创建一个新会话 "src"
    并且 向会话追加用户消息 "需求梳理"
    当 导出会话 "src" 为 JSONL 文件 "dump.jsonl"
    那么 导出文件 "dump.jsonl" 包含 "需求梳理"

  @req:r1087 @executable
  场景: import-jsonl-roundtrip
    假如 会话存储目录已初始化
    当 创建一个新会话 "src"
    并且 向会话追加用户消息 "往来内容"
    当 导出会话 "src" 为 JSONL 文件 "share.jsonl"
    当 把 JSONL 文件 "share.jsonl" 导入全新会话存储为 "src"
    那么 导入存储中会话 "src" 包含文本 "往来内容"

  @req:r1085 @executable
  场景: export-html
    假如 会话存储目录已初始化
    当 创建一个新会话 "doc"
    并且 向会话追加助手消息 "结论如下"
    当 导出会话 "doc" 为 HTML 文件 "report.html"
    那么 导出文件 "report.html" 包含 "<!doctype html>"

  @req:r1088 @executable
  场景: html-readable-blocks
    假如 会话存储目录已初始化
    当 创建一个新会话 "doc2"
    并且 向会话追加用户消息 "跑一下构建"
    并且 向会话追加 bash 执行记录（命令 "make test" 输出 "all ok"）
    并且 向会话追加助手消息 "构建通过"
    当 导出会话 "doc2" 为 HTML 文件 "blocks.html"
    那么 HTML 文件 "blocks.html" 含可读块文本 "$ make test"
    并且 HTML 文件 "blocks.html" 含可读块文本 "构建通过"

  @req:r1111 @executable
  场景: resume-fallback-cwd-rescues
    假如 会话存储目录已初始化
    当 创建存储于目录 "/definitely-missing/xylitol-cwd" 的新会话 "migrated"
    并且 向会话追加用户消息 "带上我的历史"
    那么 校验加载 "migrated" 回退 "." 成功且非空

  @req:r1094 @executable
  场景: stored-cwd-accessible-loads
    假如 会话存储目录已初始化
    当 创建存储于目录 "." 的新会话 "native"
    并且 向会话追加用户消息 "本地直接恢复"
    那么 校验加载 "native" 回退 "." 成功且非空

  @req:r1112 @executable
  场景: cwd-error-carries-both-paths
    假如 会话存储目录已初始化
    当 创建存储于目录 "/definitely-missing/xylitol-cwd" 的新会话 "lost"
    那么 校验加载 "lost" 回退 "/also-missing/fallback" 失败并提及 "/definitely-missing/xylitol-cwd" 与 "/also-missing/fallback"

  @req:r1096 @executable
  场景: tool-result-persists-tool-call-id-key
    假如 会话存储目录已初始化
    当 创建一个新会话 "tools"
    当 向会话追加关联 "call-1" 的工具结果消息 "ok"
    那么 会话 "tools" 的磁盘行含 toolCallId 且不含 toolUseId

  @req:r1108 @executable
  场景: branch-summary-empty-input-empty-output
    假如 会话存储目录已初始化
    当 调用分支摘要生成于空切点集合
    那么 分支摘要为空字符串
