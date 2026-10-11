# language: zh-CN
# capability: agent-session-store
# purpose: 会话持久化 — 基于 JSONL 文件的会话存储，含树操作、compaction、fork 与 CWD 校验。
# scope: src/infra/session/
# reviewed-through: c2837

功能: agent-session-store

  @req:r41
  规则: compaction
    当 CompactionSettings.enabled 且上下文占用超过 window−reserve（c1630）时，System 须在 agent/capabilities 回合落定后自动 compact（c1640）；不得仅依赖百分比闸或仅 TUI 轮询。
    # verified-by: llmanspec/specs/domain-compaction/domain-compaction.feature

    场景: compaction-trigger-on-reserve-exceed
      当 读取压缩触发边界
      那么 按窗口与保留阈值在会话路径触发
  @req:r1090
  规则: JSONL 存储
    SessionManager 须将会话持久化为 v7 会话目录（位于 ~/.xylitol/sessions/），目录 须由 manifest 提交指针、一个 active JSONL 段与零个或多个 sealed cold JSONL 段组成；每行仍是一个带版本标签的 JSON 对象，manifest 须是恢复时的 SSOT。

    场景: create-load
      假如 会话存储目录已初始化
      当 创建一个新会话 "test-session"
      并且 向会话追加一条消息 "用户问好"
      并且 加载会话 "test-session"
      那么 会话包含 1 条记录
      并且 记录类型为 "message"

    场景: jsonl-format
      假如 存在会话 "format-test"
      当 向会话追加 3 条不同类型的记录
      那么 JSONL 文件每行是一个完整的 JSON 对象
      并且 第一行包含 version 字段
  @req:r1097
  规则: 条目类型
    SessionManager 须支持条目类型（JSONL type 判别为 camelCase）：message、compaction、branchSummary、modelChange、thinkingLevelChange、custom、customMessage、label、sessionInfo；bang-bash 不作为顶层 type，而走 message+role=bashExecution。不得将 snake_case type 标签（如 branch_summary、bash_execution）视为合法 SSOT。

    场景: model-change
      假如 存在会话 "model-test"
      当 将会话模型从 "gpt-4o" 切换为 "claude-sonnet"
      那么 会话包含 model_change 记录
      并且 model_change 记录显示 provider 为 "anthropic"

    场景: thinking-change
      假如 存在会话 "think-test"
      当 切换思考级别为 "high"
      那么 会话包含 thinking_level_change 记录

    场景: label-set
      假如 存在会话 "label-test"
      当 向会话追加一条消息 "标记我"
      并且 为最后一条记录设置标签 "重要"
      那么 该记录的标签为 "重要"

    场景: label-clear
      假如 存在会话 "label-clear-test"
      当 向会话追加一条消息 "标签测试"
      并且 为最后一条记录设置标签 "重要"
      并且 清除该记录的标签
      那么 该记录没有标签
  @req:r1105
  规则: CRUD
    SessionManager 须支持 create、append、load、list、exists 操作。

    场景: list-sessions
      假如 存在会话 "session-a"
      并且 存在会话 "session-b"
      当 列出所有会话
      那么 结果包含 "session-a"
      并且 结果包含 "session-b"
  @req:r1106
  规则: 禁止旧 AgentPart 迁移
    Session JSONL 的 message.content AgentPart 形态以 domain-message dm1 为唯一真源；系统 不得迁移或静默解读 c646 之前的 untagged/裸字符串 content。读到此类旧 content 时 须按 agent-session as46 拒绝/跳过策略处理。文件级其它版本字段若存在 MAY 保留，但 不得借此复活旧 AgentPart 语义。

    场景: legacy-untagged-content-rejected
      假如 目录中植入含旧 untagged content 行的 v7 会话 "legacy-content"
      当 加载会话 "legacy-content"
      那么 坏行被跳过且其余条目正常加载
  @req:r1107
  规则: 会话树
    SessionManager 须维护 parent/child 会话链接：fork 创建子分支，branchSummary 引用父条目。

    场景: fork
      假如 存在会话 "parent" 包含 10 条记录
      当 在记录 5 处分叉创建会话 "child"
      那么 会话 "child" 包含 5 条记录
      并且 会话 "child" 包含一个 branch_summary 记录

    场景: tree-nav
      假如 存在会话树: "root" → "branch-a" → "branch-b"
      当 导航到 "branch-b"
      那么 上下文包含 branch-a 和 branch-b 的摘要
  @req:r1108
  规则: 分支摘要
    System 须支持 generateBranchSummary(parentEntries)，产出汇总切点前条目的 CompactionEntry。

    场景: branch-summary-empty-input-empty-output
      假如 会话存储目录已初始化
      当 调用分支摘要生成于空切点集合
      那么 分支摘要为空字符串
  @req:r1109
  规则: 写入安全
    SessionManager 写 v7 会话 须以 manifest 原子切换作为 seal 提交点：进程任意时点终止后，旧 manifest 仍 须指向可恢复的旧段，或新 manifest 须只指向完整的新段；不得将未提交的 orphan/临时段拼入会话。跨进程写入互斥见 r1950；本规则只约束 seal 原子性。

    场景: orphan-temp-segment-excluded
      假如 存在会话 "safe" 包含 2 条记录
      当 目录中植入未引用的临时段文件
      那么 会话 "safe" 包含 2 条记录
  @req:r1110
  规则: 会话 fork
    SessionManager 须支持 fork(parent_id, child_id, at_entry_id)，将父当前 leaf 路径（需要时按需读取 cold 段）复制到新子会话的 v7 active 段并 append branch_summary 条目；子会话 不得保存对父 session 段文件的跨目录引用。

    场景: fork-copies-cutoff-and-summary
      假如 存在会话 "parent" 包含 6 条记录
      当 在记录 3 处分叉创建会话 "child"
      那么 会话 "child" 包含一个 branch_summary 记录
      并且 会话 "child" 包含文本 "message 2"
      并且 会话 "child" 不含文本 "message 5"
  @req:r1091
  规则: 分支摘要实现
    fork 时 System 须生成描述被跳过条目的分支摘要：条目数、条目类型、最后 user 消息及工具调用中的 notable actions。

    场景: branch-summary-content
      假如 存在会话 "parent" 包含 6 条记录
      当 在记录 4 处分叉创建会话 "child"
      那么 会话 "child" 的 branch_summary 摘要包含 "message 4"
      并且 会话 "child" 不含文本 "message 5"
  @req:r1092
  规则: agent fork
    System 须支持 fork_session(at_entry_id)：经 SessionManager::fork() 创建新子会话并返回子 session id。

    场景: fork-session-returns-child-id
      假如 存在会话 "parent" 包含 4 条记录
      当 在记录 2 处分叉创建会话 "kid"
      那么 会话 "kid" 包含文本 "message 1"
      并且 会话 "kid" 包含一个 branch_summary 记录
  @req:r1101
  规则: fork-header-cut-entry
    fork 创建子会话时，子会话头 须记录父会话 id，且 须记录切点条目 id（fork 时所选条目，含 Before 切位时未拷入子会话的那条）；非 fork 创建的会话头 不得写入切点字段。加载缺少该切点字段的旧会话文件时 须视为无切点，不得因此失败。

    场景: fork-header-carries-cut-entry
      假如 存在会话 "parent" 包含 6 条记录
      当 在记录 3 处分叉创建会话 "child"
      那么 会话 "child" 头含父会话与切点条目 id
      并且 会话 "parent" 头不含切点字段
  @req:r1102
  规则: v7 manifest 提交
    v7 manifest 须只引用 session 目录内的完整段文件并记录 active 段与 sealed 段的逻辑顺序；compaction seal 成功后已引用的 cold 段 不得再被改写，未被 manifest 引用的临时或 orphan 段 不得影响 load、resume 或 list。

    场景: unreferenced-segment-does-not-affect-list
      假如 存在会话 "m2" 包含 2 条记录
      当 目录中植入未引用的临时段文件
      当 列出所有会话
      那么 结果包含 "m2"
      并且 会话 "m2" 包含 2 条记录
  @req:r1103
  规则: 冷段按需恢复
    resume、load_leaf_branch 与 session context 构造 须先读取 manifest 与 active 段，仅在当前 leaf 的 parentId/分支或会话级配对需要时按需读取命中的 cold 段；不得为普通 resume 无条件解析全部 cold 段。被 manifest 引用且带合法 sidecar 的 sealed 段 须以 sidecar 索引（entryIds / doneBashIds）先筛候选段；sidecar 缺失、损坏或候选未命中时 须回退读取未读 sealed 段，不得因索引漏报（false negative）截断 leaf 分支或误报 interrupted bash。完整导出或 inspect 可显式读取完整逻辑条目流。

    场景: cold-segment-restore-leaf-correct
      假如 会话有 50 个轮次
      当 触发压缩保留最近 10 轮
      并且 向会话追加一条消息 "压缩后新消息"
      当 加载会话 "compaction-retain"
      那么 会话 JSONL 包含 CompactionEntry
      并且 会话 "compaction-retain" 包含文本 "压缩后新消息"
  @req:r1900
  规则: sealed-sidecar-index
    每个被 v7 manifest 引入的 sealed 段 MAY 生成不可变 JSON sidecar index，记录该段全部非空 entryId 与状态为 done 的 bash id（输出稳定排序去重）；manifest 的 sealed segment 描述 须支持可选 camelCase `indexPath` 引用它。若 sidecar 存在，则侧载构建 须与 seal 的同一批 prefix 条目一致，不得改变逻辑 JSONL export/import 输出形状；旧 manifest 无 `indexPath`、sidecar 缺失或损坏时 须回退读取对应 sealed JSONL，不得因索引损坏产生 false negative。

    场景: sidecar-corrupt-falls-back-to-scan
      假如 会话有 50 个轮次
      当 触发压缩保留最近 10 轮
      当 破坏该会话的 sealed sidecar 索引文件
      当 加载会话 "compaction-retain"
      那么 会话 JSONL 包含 CompactionEntry
  @req:r1901
  规则: resume-projection-llm-api
    resume/context 只从可解析 v7 session 的最新 leaf 分支恢复，不得为 legacy 或不可解析存储降级恢复路径；构建的 session context 须继续投影 LLM API 相关信息（thinkingLevel 与 model 选择，来自 leaf 分支上的 modelChange / thinkingLevelChange 条目）且不做钳制、不追加抵消条目；sidecar 与 done-bash 端口只优化 bash 配对数据来源，不得破坏投影的既有可观察语义。

    场景: resume-projects-llm-api-verbatim
      假如 存在会话 "proj"
      当 将会话模型从 "gpt-4o" 切换为 "claude-sonnet"
      当 切换思考级别为 "high"
      当 加载会话 "proj"
      那么 恢复投影模型为 "claude-sonnet" 且思考档为 "high" 且无抵消条目
  @req:r1104
  规则: 旧格式边界
    仅存在旧 v6/v5 或未知单文件 `{id}.jsonl` 且无 v7 manifest 时，System 不得自动迁移、不得静默删除、不得创建同 id 的新 v7 session 覆盖旧文件；自动恢复与写入入口 须返回可操作的不支持错误并保留源文件；用户显式 delete 仍可清理；list 须跳过该条目并记录有限诊断，不得把它当作可 resume 的 v7 session。

    场景: list-skips-legacy-only-session
      假如 会话存储目录已初始化
      当 目录中植入旧格式会话文件 "legacy-only"
      当 列出所有会话
      那么 结果不包含 "legacy-only"
  @req:r1094
  规则: 会话 CWD 校验
    从磁盘加载会话时 SessionManager 须校验存储 CWD 存在且可访问；创建/导入会话时同样 须校验工作目录可访问；不可用时返回可操作错误信息。

    场景: stored-cwd-accessible-loads
      假如 会话存储目录已初始化
      当 创建存储于目录 "." 的新会话 "native"
      并且 向会话追加用户消息 "本地直接恢复"
      那么 校验加载 "native" 回退 "." 成功且非空
  @req:r1095
  规则: session-entry-camelcase-v7
    Session persisted JSONL 须以 camelCase 为唯一磁盘格式 SSOT（JS/TS favor；非 pi snake entry type）：v7 段外壳字段含 parentId、parentSession、firstKeptEntryId 等；SessionEntry type 判别为 message、compaction、branchSummary、modelChange、thinkingLevelChange、custom、customMessage、label、sessionInfo。新写入的 bang-bash 须使用 type=message 且 message.role=bashExecution，不得再写出顶层 type=bashExecution 或 bash_execution。新写入的 header.version 须等于 SESSION_VERSION（7），manifest 的 formatVersion 须同为 7。不得再写出 parent_id 或 version≤5 作为新会话真源。旧格式文件 不得被自动迁移为 v7（见 s26 旧格式边界）、不得提供长期 serde alias 或旧顶层 bash 升格为合法上下文。

    场景: v7-shell-camelcase-and-bash-message
      假如 存在会话 "fmt7"
      当 向会话追加 bash 执行记录（命令 "make test" 输出 "all ok"）
      那么 新会话磁盘头 version 为 7 且外壳字段为 camelCase
      并且 bash 记录行 type 为 message 且 role 为 bashExecution
  @req:r1096
  规则: tool-result-tool-call-id
    role=toolResult 的 AgentMessage 须以 toolCallId 键持久化工具调用关联 id（对齐 pi）；不得写出 toolUseId。content 内 不得再嵌入 type=toolResult 的 AgentPart（工具结果只走独立 message 行）。

    场景: tool-result-persists-tool-call-id-key
      假如 会话存储目录已初始化
      当 创建一个新会话 "tools"
      当 向会话追加关联 "call-1" 的工具结果消息 "ok"
      那么 会话 "tools" 的磁盘行含 toolCallId 且不含 toolUseId
  @req:r1098
  规则: session-load-skip-warn
    load（及同源逐行解析）遇到无法按最新 SSOT 解析的行（坏 JSON、未知 type、非 SSOT snake type 如 bash_execution、旧 untagged content 导致无法投影）时 须跳过该行并记录可观测 warn；同一 load/list 操作内明文 warn 须至多 3 条，超出后 须以单条省略标记（如 ...）收敛，不得刷屏。v7 manifest/header version 不等于 SESSION_VERSION（7）时 须拒绝将其视为合法最新会话（返回可操作错误）；旧格式单文件不再自动迁移（见 s26 旧格式边界），不得静默把 v5 及更早版本 migrate 后当成功。TUI resume、print/CLI --session 与 SessionManager 须共用此策略。

    场景: load-skips-unparseable-lines
      假如 目录中植入含坏 JSON 行的 v7 会话 "mixed"
      当 加载会话 "mixed"
      那么 坏行被跳过且其余条目正常加载
  @req:r1099
  规则: list-sessions-resilient
    list_sessions（或等价枚举）遇到单个 v7 manifest/段不可读、非最新或解析失败时 须直接跳过该 session（不迁移、不修复、不解析展示）并继续枚举其余会话；发现旧格式 `{id}.jsonl`（无 v7 manifest）时 须直接跳过并记录有限诊断，不得尝试识别或迁移；不得因单 session 失败而使整表 list/resume 面板失败。

    场景: list-resilient-to-corrupt-file
      假如 存在会话 "good" 包含 2 条记录
      当 目录中植入损坏的会话文件 broken.jsonl
      当 列出所有会话
      那么 结果包含 "good"
  @req:r1085
  规则: 导出 HTML
    System 须支持将会话导出为 HTML 文件。

    场景: export-html
      假如 会话存储目录已初始化
      当 创建一个新会话 "doc"
      并且 向会话追加助手消息 "结论如下"
      当 导出会话 "doc" 为 HTML 文件 "report.html"
      那么 导出文件 "report.html" 包含 "<!doctype html>"
  @req:r1086
  规则: 导出 JSONL
    System 须支持将会话导出为 JSONL 文件。

    场景: export-jsonl
      假如 会话存储目录已初始化
      当 创建一个新会话 "src"
      并且 向会话追加用户消息 "需求梳理"
      当 导出会话 "src" 为 JSONL 文件 "dump.jsonl"
      那么 导出文件 "dump.jsonl" 包含 "需求梳理"
  @req:r1087
  规则: 导入 JSONL
    System 须支持从 JSONL 文件导入会话为新会话。

    场景: import-jsonl-roundtrip
      假如 会话存储目录已初始化
      当 创建一个新会话 "src"
      并且 向会话追加用户消息 "往来内容"
      当 导出会话 "src" 为 JSONL 文件 "share.jsonl"
      当 把 JSONL 文件 "share.jsonl" 导入全新会话存储为 "src"
      那么 导入存储中会话 "src" 包含文本 "往来内容"
  @req:r1088
  规则: HTML 工具渲染
    HTML 导出 须将 tool 与 assistant 内容渲染为可读块。

    场景: html-readable-blocks
      假如 会话存储目录已初始化
      当 创建一个新会话 "doc2"
      并且 向会话追加用户消息 "跑一下构建"
      并且 向会话追加 bash 执行记录（命令 "make test" 输出 "all ok"）
      并且 向会话追加助手消息 "构建通过"
      当 导出会话 "doc2" 为 HTML 文件 "blocks.html"
      那么 HTML 文件 "blocks.html" 含可读块文本 "$ make test"
      并且 HTML 文件 "blocks.html" 含可读块文本 "构建通过"
  @req:r1089
  @req:r1111
  规则: CWD 校验
    恢复会话时 System 须校验会话头存储的 cwd 在磁盘可用；不可用时 须尝试 fallback cwd，其一可用即继续。

    场景: resume-fallback-cwd-rescues
      假如 会话存储目录已初始化
      当 创建存储于目录 "/definitely-missing/xylitol-cwd" 的新会话 "migrated"
      并且 向会话追加用户消息 "带上我的历史"
      那么 校验加载 "migrated" 回退 "." 成功且非空
  @req:r1112
  规则: CWD 错误
    两者均不可用时 System 须返回可操作校验错误，消息 须同时携带存储 cwd 与 fallback cwd；面向用户的 CWD 错误信息 须即该可操作校验错误文本（引导用户落到 fallback cwd）。

    场景: cwd-error-carries-both-paths
      假如 会话存储目录已初始化
      当 创建存储于目录 "/definitely-missing/xylitol-cwd" 的新会话 "lost"
      那么 校验加载 "lost" 回退 "/also-missing/fallback" 失败并提及 "/definitely-missing/xylitol-cwd" 与 "/also-missing/fallback"
  @req:r1114
  规则: CWD 集成
    CLI 与 RPC 模式 须在恢复会话前完成同一校验。
    # verified-by: llmanspec/specs/agent-session-store/agent-session-store.feature

    场景: cwd-check-before-restore
      当 读取恢复会话校验顺序
      那么 恢复前完成同一 CWD 校验
  @req:r1115
  规则: SessionStore 端口实现
    infra 层的 SessionManager 须实现 protocol 的 XySessionStore 端口（append、load、load_context、exists），仅覆盖 agent 循环与 compaction 所需；完整会话面（fork、navigate、export）MAY 留在具体结构体供组合根直接使用。
    # verified-by: src/AGENTS.md

    场景: session-manager-implements-port
      当 读取会话存储端口实现
      那么 infra SessionManager 实现协议端口
  @req:r1117
  规则: 日志访问
    会话 store 须暴露 read_recent(session_id, limit) -> Vec<Event> 供 server journal；journal 容量 须可配置（默认 10000）。
    # verified-by: llmanspec/specs/server-core/server-core.feature

    场景: journal-read-recent-boundary
      当 读取会话日志访问接口
      那么 read_recent 暴露给 server journal
  @req:r1116
  规则: ExportIo 实现
    System 须提供导出 I/O 端口实现（StdExportIo 或等价，tokio::fs 读写）；组合根 须将导出协作器经该端口注入。
    # verified-by: llmanspec/specs/agent-session-store/agent-session-store.feature

    场景: export-io-port-injected
      当 读取导出 I/O 装配
      那么 StdExportIo 经端口注入组合根
  @req:r1100
  规则: 磁盘格式时间戳
    Session 磁盘格式时间戳（header.timestamp 与条目壳 timestamp）须为 u64 unix 毫秒数，与 message 内 timestamp 同基准；不得写出 RFC3339 字符串作为磁盘格式时间戳。展示面需要人类可读时间时 须在展示边格式化。

    场景: timestamps-u64-ms
      假如 存在会话 "ts1" 包含 2 条记录
      当 加载会话 "ts1"
      那么 会话 "ts1" 的 JSONL 时间戳均为 u64 毫秒
  @req:r1093
  规则: 延迟持久化至 assistant
    Persisted SessionManager（非 in_memory backend）须对齐 pi：首条 assistant 消息写入前，会话条目可仅保留在内存；首条 assistant append 时 须创建磁盘 jsonl（若尚不存在）并刷出此前挂起条目（含 header 与先前 user 等）；其后每次 append 须落盘。InMemory backend 不得写文件。不得在仅有 header、用户又放弃对话时强制留下无消息垃圾文件作为唯一策略（允许 create 仍写 header，但 deferred 刷出路径 须存在并可测）。

    场景: delayed-header-flush-on-first-append
      假如 会话存储目录已初始化
      当 创建一个新会话 "lazy"
      那么 会话 "lazy" 的磁盘 JSONL 尚不存在
      当 向会话追加用户消息 "hello"
      那么 会话 "lazy" 的磁盘 JSONL 已存在且包含 "hello"
  @req:r1950
  规则: persisted-session-kernel-write-lock
    已物化（磁盘上有提交指针）的持久化会话，跨进程写入 须经内核排他占用：POSIX 须 flock 该会话目录内 `session.lock`（释放 不得 unlink；不得锁会原子改写的提交指针）；Windows 须用由该锁路径派生的命名内核对象。持有进程退出后后继 须可写入。内存后端 不得要求该锁。

    场景: second-process-write-open-conflicts
      假如 已物化会话的写占用被进程 A 持有
      当 进程 B 打开同一会话写入
      那么 进程 B 失败且不得追加

    场景: lock-releases-on-holder-drop
      假如 进程 A 持有已物化会话写占用后退出
      当 进程 B 打开同一会话写入
      那么 进程 B 可以追加
