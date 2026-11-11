# language: zh-CN
# purpose: 会话持久化 — 基于 JSONL 文件的会话存储，含树操作、compaction、fork 与 CWD 校验。

功能: agent-session-store

  @req:r41
  规则: compaction
    当 CompactionSettings.enabled 且上下文占用超过 window−reserve（c1630）时，System 须在 agent/capabilities 回合落定后自动 compact（c1640）；不得仅依赖百分比闸或仅 TUI 轮询。

    场景: 压缩触发时预留超出
      当 读取压缩触发边界
      那么 按窗口与保留阈值在会话路径触发

  @req:r1090
  规则: 追加日志存储
    SessionManager 须将会话持久化为追加式 JSONL 日志目录（位于 ~/.xylitol/sessions/<id>/log/）：每行一个 JSON entry（含 seq 可选字段），首行 header 内嵌 format=log-v1；manifest 与 index 均为可丢弃缓存，恢复真值只来自 log 行。轮转只在 turn 边界或压缩点发生；不得把 manifest 当作恢复 SSOT。

    场景: 创建加载
      假如 会话存储目录已初始化
      当 创建一个新会话 "test-session"
      并且 向会话追加一条消息 "用户问好"
      并且 加载会话 "test-session"
      那么 会话包含 1 条记录
      并且 记录类型为 "message"

    场景: JSONL格式
      假如 存在会话 "format-test"
      当 向会话追加 3 条不同类型的记录
      那么 JSONL 文件每行是一个完整的 JSON 对象
      并且 第一行包含 version 字段

  @req:r1097
  规则: 条目类型
    SessionManager 须支持条目类型（JSONL type 判别为 camelCase）：message、compaction、branchSummary、modelChange、thinkingLevelChange、custom、customMessage、label、sessionInfo；bang-bash 不作为顶层 type，而走 message+role=bashExecution。不得将 snake_case type 标签（如 branch_summary、bash_execution）视为合法 SSOT。

    场景: 模型变更
      假如 存在会话 "model-test"
      当 将会话模型从 "gpt-4o" 切换为 "claude-sonnet"
      那么 会话包含 model_change 记录
      并且 model_change 记录显示 provider 为 "anthropic"

    场景: 思考变更
      假如 存在会话 "think-test"
      当 切换思考级别为 "high"
      那么 会话包含 thinking_level_change 记录

    场景: 标签设置
      假如 存在会话 "label-test"
      当 向会话追加一条消息 "标记我"
      并且 为最后一条记录设置标签 "重要"
      那么 该记录的标签为 "重要"

    场景: 标签清除
      假如 存在会话 "label-clear-test"
      当 向会话追加一条消息 "标签测试"
      并且 为最后一条记录设置标签 "重要"
      并且 清除该记录的标签
      那么 该记录没有标签

  @req:r1105
  规则: CRUD
    SessionManager 须支持 create、append、load、list、exists 操作。

    场景: 列表会话
      假如 存在会话 "session-a"
      并且 存在会话 "session-b"
      当 列出所有会话
      那么 结果包含 "session-a"
      并且 结果包含 "session-b"

  @req:r1106
  规则: 禁止旧 AgentPart 迁移
    Session JSONL 的 message.content AgentPart 形态以 domain-message dm1 为唯一真源；系统 不得迁移或静默解读 c646 之前的 untagged/裸字符串 content。读到此类旧 content 时 须按 agent-session as46 拒绝/跳过策略处理。文件级其它版本字段若存在 MAY 保留，但 不得借此复活旧 AgentPart 语义。

    场景: 旧版无标签内容拒绝
      假如 目录中植入含旧 untagged content 行的 log-v1 会话 "legacy-content"
      当 加载会话 "legacy-content"
      那么 坏行被跳过且其余条目正常加载

  @req:r1107
  规则: 会话树
    SessionManager 须维护 parent/child 会话链接：fork 创建子分支，branchSummary 引用父条目。

    场景: 分叉
      假如 存在会话 "parent" 包含 10 条记录
      当 在记录 5 处分叉创建会话 "child"
      那么 会话 "child" 包含一个 branch_summary 记录

    场景: 树导航
      假如 存在会话树: "root" → "branch-a" → "branch-b"
      当 导航到 "branch-b"
      那么 上下文包含 branch-a 和 branch-b 的摘要

  @req:r1108
  规则: 分支摘要
    System 须支持 generateBranchSummary(parentEntries)，产出汇总切点前条目的 CompactionEntry。fork 时 System 须生成描述被跳过条目的分支摘要：条目数、条目类型、最后 user 消息及工具调用中的 notable actions。

    场景: 分支摘要空输入空输出
      假如 会话存储目录已初始化
      当 调用分支摘要生成于空切点集合
      那么 分支摘要为空字符串

    场景: 分支摘要内容
      假如 存在会话 "parent" 包含 6 条记录
      当 在记录 4 处分叉创建会话 "child"
      那么 会话 "child" 的 branch_summary 摘要包含 "message 4"
      并且 会话 "child" 不含文本 "message 5"

  @req:r1109
  规则: 追加写入安全
    每行 append 须是单行原子写：进程任意时点终止后，已 fsync 的完整行 须可恢复，尾部半行 须在打开快检时截断；不得把未完成的行拼入会话。跨进程写入互斥见 r1950。

    场景: 孤儿文件排除
      假如 存在会话 "safe" 包含 2 条记录
      当 目录中植入未引用的临时段文件
      那么 会话 "safe" 包含 2 条记录

  @req:r1110
  规则: 会话 fork
    SessionManager 须支持 fork(parent_id, child_id, at_entry_id)，将父当前 leaf 路径以 copy-on-write 引用到子会话：子会话头 须记录 parentSession 与 forkAtEntryId，并 append branch_summary；子会话 不得复制父条目或重写父条目 id。加载子会话时 须沿父会话 log 读取祖先条目（树视图穿透），LLM 视图 须沿 parent 链折叠。System 须支持 fork_session(at_entry_id)：经 SessionManager::fork() 创建新子会话并返回子 session id。非 fork 创建的会话头 不得写入切点字段；加载缺少该切点字段的旧会话文件时 须视为无切点，不得因此失败。

    场景: 分叉复制裁切点与摘要
      假如 存在会话 "parent" 包含 6 条记录
      当 在记录 3 处分叉创建会话 "child"
      那么 会话 "child" 包含一个 branch_summary 记录
      并且 会话 "child" 包含文本 "message 2"
      并且 会话 "child" 不含文本 "message 5"

    场景: 分叉会话返回子项标识
      假如 存在会话 "parent" 包含 4 条记录
      当 在记录 2 处分叉创建会话 "kid"
      那么 会话 "kid" 包含文本 "message 1"
      并且 会话 "kid" 包含一个 branch_summary 记录

    场景: 分叉头携带裁切入口
      假如 存在会话 "parent" 包含 6 条记录
      当 在记录 3 处分叉创建会话 "child"
      那么 会话 "child" 头含父会话与切点条目 id
      并且 会话 "parent" 头不含切点字段

  @req:r1102
  规则: manifest 缓存
    manifest 须只作可丢弃缓存（记录 activeFile、tailBytes、lineCount、nextSeq、leafId、rotation、index）；损坏或缺失 须从 log 重建，不得当作空会话；未被 manifest 引用的临时文件 不得影响 load、resume 或 list。

    场景: 未引用文件不影响列表
      假如 存在会话 "m2" 包含 2 条记录
      当 目录中植入未引用的临时段文件
      当 列出所有会话
      那么 结果包含 "m2"
      并且 会话 "m2" 包含 2 条记录

  @req:r1103
  规则: 投影视图
    load_leaf_branch 须从 leaf 沿 parent 链折叠，碰 Compaction 截断（摘要作前缀）；树视图 须全量穿透；分页视图 须按 turn 或 entry 粒度返回窗口。不得为普通 resume 无条件解析无关分支。完整导出或 inspect 可显式读取完整逻辑条目流。

    场景: 压缩保留叶子投影
      假如 会话有 50 个轮次
      当 触发压缩保留最近 10 轮
      并且 向会话追加一条消息 "压缩后新消息"
      当 加载会话 "compaction-retain"
      那么 会话 JSONL 包含 CompactionEntry
      并且 会话 "compaction-retain" 包含文本 "压缩后新消息"

  @req:r1900
  规则: 稀疏锚点索引
    会话目录 index/ 下 MAY 为每个 log 文件生成稀疏锚点（每 64 行一条，记录 entryId 与字节偏移）；index 缺失、损坏或未建时读路径 须回退线性扫描，不得因索引损坏产生 false negative；重建归维护任务，不得改变逻辑 JSONL export/import 输出形状。

    场景: 锚点索引损坏回落扫描
      假如 会话有 50 个轮次
      当 触发压缩保留最近 10 轮
      当 破坏该会话的稀疏锚点索引文件
      当 加载会话 "compaction-retain"
      那么 会话 JSONL 包含 CompactionEntry

  @req:r1901
  规则: resume-projection-llm-api
    resume/context 只从可解析 log-v1 session 的最新 leaf 分支恢复，不得为 legacy 或不可解析存储降级恢复路径；构建的 session context 须继续投影 LLM API 相关信息（thinkingLevel 与 model 选择，来自 leaf 分支上的 modelChange / thinkingLevelChange 条目）且不做钳制、不追加抵消条目；稀疏索引与 done-bash 端口只优化查询数据来源，不得破坏投影的既有可观察语义。

    场景: 续跑投影模型API原文
      假如 存在会话 "proj"
      当 将会话模型从 "gpt-4o" 切换为 "claude-sonnet"
      当 切换思考级别为 "high"
      当 加载会话 "proj"
      那么 恢复投影模型为 "claude-sonnet" 且思考档为 "high" 且无抵消条目

  @req:r1104
  规则: 旧格式边界
    仅存在旧 v6/v5 或未知单文件 `{id}.jsonl` 且无 log 目录时，System 不得自动迁移、不得静默删除、不得创建同 id 的新会话覆盖旧文件；自动恢复与写入入口 须返回可操作的不支持错误并保留源文件；用户显式 delete 仍可清理；list 须跳过该条目并记录有限诊断，不得把它当作可 resume 的会话。

    场景: 列表跳过旧版仅会话
      假如 会话存储目录已初始化
      当 目录中植入旧格式会话文件 "legacy-only"
      当 列出所有会话
      那么 结果不包含 "legacy-only"

  @req:r1094
  规则: CWD 校验与回退
    从磁盘加载会话时 SessionManager 须校验存储 CWD 存在且可访问；创建/导入会话时同样 须校验工作目录可访问；不可用时 须尝试 fallback cwd，其一可用即继续。两者均不可用时 须返回可操作校验错误，消息 须同时携带存储 cwd 与 fallback cwd；面向用户的 CWD 错误信息 即该可操作校验错误文本（引导用户落到 fallback cwd）。CLI 与 RPC 模式 须在恢复会话前完成同一校验。

    场景: 已存工作目录可访问加载
      假如 会话存储目录已初始化
      当 创建存储于目录 "." 的新会话 "native"
      并且 向会话追加用户消息 "本地直接恢复"
      那么 校验加载 "native" 回退 "." 成功且非空

    场景: 续跑回退工作目录挽救
      假如 会话存储目录已初始化
      当 创建存储于目录 "/definitely-missing/xylitol-cwd" 的新会话 "migrated"
      并且 向会话追加用户消息 "带上我的历史"
      那么 校验加载 "migrated" 回退 "." 成功且非空

    场景: 工作目录错误携带两者路径
      假如 会话存储目录已初始化
      当 创建存储于目录 "/definitely-missing/xylitol-cwd" 的新会话 "lost"
      那么 校验加载 "lost" 回退 "/also-missing/fallback" 失败并提及 "/definitely-missing/xylitol-cwd" 与 "/also-missing/fallback"

    场景: 工作目录校验之前恢复
      当 读取恢复会话校验顺序
      那么 恢复前完成同一 CWD 校验

  @req:r1095
  规则: 日志条目驼峰
    Session persisted JSONL 须以 camelCase 为唯一磁盘格式 SSOT：log-v1 header 行内嵌 format 字段，每行含 seq；SessionEntry type 判别为 message、compaction、branchSummary、modelChange、thinkingLevelChange、custom、customMessage、label、sessionInfo。新写入的 bang-bash 须使用 type=message 且 message.role=bashExecution，不得再写出顶层 type=bashExecution 或 bash_execution。新写入的 header.version 须等于 SESSION_VERSION（7）。不得再写出 parent_id 或 version≤5 作为新会话真源；旧格式文件 不得被自动迁移为 log-v1（见 s26 旧格式边界）、不得提供长期 serde alias 或旧顶层 bash 升格为合法上下文。

    场景: 日志壳驼峰与bash消息
      假如 存在会话 "fmt7"
      当 向会话追加 bash 执行记录（命令 "make test" 输出 "all ok"）
      那么 新会话磁盘头 version 为 7 且外壳字段为 camelCase
      并且 bash 记录行 type 为 message 且 role 为 bashExecution

  @req:r1096
  规则: tool-result-tool-call-id
    role=toolResult 的 AgentMessage 须以 toolCallId 键持久化工具调用关联 id（对齐 pi）；不得写出 toolUseId。content 内 不得再嵌入 type=toolResult 的 AgentPart（工具结果只走独立 message 行）。

    场景: 工具结果持久化工具调用标识键
      假如 会话存储目录已初始化
      当 创建一个新会话 "tools"
      当 向会话追加关联 "call-1" 的工具结果消息 "ok"
      那么 会话 "tools" 的磁盘行含 toolCallId 且不含 toolUseId

  @req:r1098
  规则: session-load-skip-warn
    load（及同源逐行解析）遇到无法按最新 SSOT 解析的行（坏 JSON、未知 type、非 SSOT snake type 如 bash_execution、旧 untagged content 导致无法投影）时 须跳过该行并记录可观测 warn；同一 load/list 操作内明文 warn 须至多 3 条，超出后 须以单条省略标记（如 ...）收敛，不得刷屏。尾部半行 须截断。log-v1 header 的 version 不等于 SESSION_VERSION（7）或 format 不等于 log-v1 时 须拒绝将其视为合法最新会话（返回可操作错误）；旧格式单文件不再自动迁移（见 s26 旧格式边界），不得静默把 v5 及更早版本 migrate 后当成功。TUI resume、print/CLI --session 与 SessionManager 须共用此策略。

    场景: 加载跳过无法解析行
      假如 目录中植入含坏 JSON 行的 log-v1 会话 "mixed"
      当 加载会话 "mixed"
      那么 坏行被跳过且其余条目正常加载

  @req:r1099
  规则: list-sessions-resilient
    list_sessions（或等价枚举）遇到单个 log 文件不可读、头行非法或解析失败时 须直接跳过该 session（不迁移、不修复、不解析展示）并继续枚举其余会话；发现旧格式 `{id}.jsonl`（无 log 目录）时 须直接跳过并记录有限诊断，不得尝试识别或迁移；不得因单 session 失败而使整表 list/resume 面板失败。

    场景: 列表韧性至损坏文件
      假如 存在会话 "good" 包含 2 条记录
      当 目录中植入损坏的会话文件 broken.jsonl
      当 列出所有会话
      那么 结果包含 "good"

  @req:r1085
  规则: 导出 HTML
    System 须支持将会话导出为 HTML 文件。

    场景: 导出HTML
      假如 会话存储目录已初始化
      当 创建一个新会话 "doc"
      并且 向会话追加助手消息 "结论如下"
      当 导出会话 "doc" 为 HTML 文件 "report.html"
      那么 导出文件 "report.html" 包含 "<!doctype html>"

  @req:r1086
  规则: 导出 JSONL
    System 须支持将会话导出为 JSONL 文件。

    场景: 导出JSONL
      假如 会话存储目录已初始化
      当 创建一个新会话 "src"
      并且 向会话追加用户消息 "需求梳理"
      当 导出会话 "src" 为 JSONL 文件 "dump.jsonl"
      那么 导出文件 "dump.jsonl" 包含 "需求梳理"

  @req:r1087
  规则: 导入 JSONL
    System 须支持从 JSONL 文件导入会话为新会话。

    场景: 导入JSONL往返
      假如 会话存储目录已初始化
      当 创建一个新会话 "src"
      并且 向会话追加用户消息 "往来内容"
      当 导出会话 "src" 为 JSONL 文件 "share.jsonl"
      当 把 JSONL 文件 "share.jsonl" 导入全新会话存储为 "src"
      那么 导入存储中会话 "src" 包含文本 "往来内容"

  @req:r1088
  规则: HTML 工具渲染
    HTML 导出 须将 tool 与 assistant 内容渲染为可读块。

    场景: HTML可读块
      假如 会话存储目录已初始化
      当 创建一个新会话 "doc2"
      并且 向会话追加用户消息 "跑一下构建"
      并且 向会话追加 bash 执行记录（命令 "make test" 输出 "all ok"）
      并且 向会话追加助手消息 "构建通过"
      当 导出会话 "doc2" 为 HTML 文件 "blocks.html"
      那么 HTML 文件 "blocks.html" 含可读块文本 "$ make test"
      并且 HTML 文件 "blocks.html" 含可读块文本 "构建通过"

  @req:r1089

  @req:r1115
  规则: SessionStore 端口实现
    SessionStore 端口必须覆盖 append、load、load_context、exists；完整会话面（fork、navigate、export）可留在具体结构体。 由 SessionStore 端口契约单测覆盖，不单独扩 BDD step。

  @req:r1117
  规则: 日志访问
    会话 store 须暴露 read_recent(session_id, limit) -> Vec<Event> 供 server journal；journal 容量 须可配置（默认 10000）。

    场景: 日志读取最近边界
      当 读取会话日志访问接口
      那么 read_recent 暴露给 server journal

  @req:r1116
  规则: ExportIo 实现
    System 须提供导出 I/O 端口实现（StdExportIo 或等价，tokio::fs 读写）；组合根 须将导出协作器经该端口注入。

    场景: 导出IO端口注入
      当 读取导出 I/O 装配
      那么 StdExportIo 经端口注入组合根

  @req:r1100
  规则: 磁盘格式时间戳
    Session 磁盘格式时间戳（header.timestamp 与条目壳 timestamp）须为 u64 unix 毫秒数，与 message 内 timestamp 同基准；不得写出 RFC3339 字符串作为磁盘格式时间戳。展示面需要人类可读时间时 须在展示边格式化。

    场景: 时间戳u64毫秒
      假如 存在会话 "ts1" 包含 2 条记录
      当 加载会话 "ts1"
      那么 会话 "ts1" 的 JSONL 时间戳均为 u64 毫秒

  @req:r1093
  规则: 写路径与 fsync
    Persisted SessionManager 须在 create 时写入 header 行，append 时立即追加一行；fsync 粒度须为 turn 边界批量（assistant 最终答复、用户消息、compaction 提交），崩溃最多丢正在进行的 turn，尾部半行须在打开快检时截断。InMemory backend 不得写文件。

    场景: 创建即写头且首次追加可读
      假如 会话存储目录已初始化
      当 创建一个新会话 "lazy"
      那么 会话 "lazy" 的磁盘 JSONL 已存在且包含 "session"
      当 向会话追加用户消息 "hello"
      那么 会话 "lazy" 的磁盘 JSONL 已存在且包含 "hello"

  @req:r1950
  规则: persisted-session-kernel-write-lock
    已物化（磁盘上有提交指针）的持久化会话，跨进程写入 须经内核排他占用：POSIX 须 flock 该会话目录内 `session.lock`（释放 不得 unlink；不得锁会原子改写的提交指针）；Windows 须用由该锁路径派生的命名内核对象。持有进程退出后后继 须可写入。内存后端 不得要求该锁。

    场景: 第二次进程写入打开冲突
      假如 已物化会话的写占用被进程 A 持有
      当 进程 B 打开同一会话写入
      那么 进程 B 失败且不得追加

    场景: 锁释放时持有者丢弃
      假如 进程 A 持有已物化会话写占用后退出
      当 进程 B 打开同一会话写入
      那么 进程 B 可以追加

  @req:r2938
  规则: 会话条目分页读取
    XySessionStore 须提供分页读取入口：limit 缺省返回全量；给定 limit 时从最新一条向前取该条数；给定 before_entry_id 时取该条目之前的更早条目；未知 before_entry_id 须返回空页。分页结果 须与全量读取后切片逐条一致。持久化后端 须按 manifest 段首尾条目 id 与 sidecar 索引跳过窗口外的整段，不得为取一页而读取全部段；窗口跨越的段 须读取并按需切片。has_more 须表示窗口之外仍有更早条目。段级跳过与读取段计数 由存储层单测覆盖，不为内部读取计数单开 BDD。

    场景: 分页与全量切片一致
      假如 会话有 N 条条目
      当 以 limit 与 before_entry_id 读取
      那么 结果与全量读取后切片逐条一致且未知 before 返回空页

  @req:r2941
  规则: todo-checkpoint-entry
    todo checkpoint 须以 custom_type = "todo_checkpoint" 的 CustomEntry 存入会话，沿用既有 v7 磁盘格式，不改布局；data 至少含 steps（id/content/status 与可选 verification/output_ref）、可选 last_verified_artifact / last_failure / resume_cursor（step_id 与 attempt）以及 updated_at。读取 须取叶分支上最新一条；缺失或损坏时 须按无 checkpoint 处理，不得中断。会话加载时 须把旧 custom_type = "plan_checkpoint" 条目一次性迁移为 "todo_checkpoint"（只追加、幂等，重复加载不得重复追加）；读取 优先新名，旧名 仅作迁移输入，新写入 不得再使用旧名。todo checkpoint 与 agent_todo 并存：todo 仍是人可见台账，checkpoint 是恢复状态，两者 不得互相替代。写入 须只追加，不得覆盖或删除历史条目。往返、最新选取与迁移幂等 由 SDK 单测覆盖。

    场景: todo checkpoint 落为 CustomEntry
      假如 todo 状态发生变化
      当 写入 checkpoint
      那么 以 custom_type 为 todo_checkpoint 的 CustomEntry 保存且沿用 v7 布局

  @req:r2945
  规则: workflow-artifact
    workflow 须以 custom_type = "workflow" 的 CustomEntry 存入会话，沿用既有 v7 磁盘格式；data 含 goal、steps（id/description/depends_on/inputs/outputs/tools/risk/verification/rollback/requires_human/status/output_ref）、可选 resume_cursor 与 updated_at。读取 须取叶分支最新一条；会话加载时 须把旧 custom_type = "plan" 条目一次性迁移为 "workflow"（只追加、幂等，重复加载不得重复追加），读取 优先新名，旧名 仅作迁移输入；新写入 不得再使用旧名。校验 须保证 id 唯一、依赖存在且无环。重规划约束：已完成步骤 不得删除、降级或改写 description / verification / depends_on；只有失败步骤及其下游闭包可结构性变更；无失败时 只允许 status / output_ref 变化。workflow 与 agent_todo、todo_checkpoint 并存，不得互相替代。DAG 校验、重规划约束与迁移幂等 由 SDK / 中间件单测覆盖。

    场景: workflow 条目保存步骤图
      假如 模型调用 workflow_write 写入 workflow
      当 读取会话叶分支
      那么 workflow CustomEntry 含 goal 与 steps 且沿用 v7 布局

  @req:r2967
  规则: workflow-tool-write
    workflow 须由 `workflow_write` 工具写入 `custom_type = "workflow"` 的 CustomEntry，格式与既有条目一致；中间件 不得再解析 assistant 文本写 workflow。旧会话中的 workflow CustomEntry 仍按原格式读取，不做文本迁移。由插件单测覆盖。

  @req:r2970
  规则: nested-workflow-entry
    workflow CustomEntry 的 steps 可含 `sub_workflow`（递归同构）；`resume_cursor` 为 `/` 分隔的步骤路径。读取 须取叶分支最新一条并按路径解析；不得为子 workflow 新建独立条目。由 SDK 单测覆盖。
  @req:r3210
  规则: 会话图边日志
    会话存储 须维护只追加的边日志；每条边记录含 edgeKind（parent / context / fork）、from、to、order、enabled 与 timestamp。禁用一条边 须追加 enabled=false 的新记录，不得删除节点或既有边；上下文投影 须按 order 排序取每个 (from, to) 的最新记录。

  @req:r3211
  规则: 分支与父引用
    会话 manifest 须记录 branches、parent 引用与 workspaceRef；parent 引用 须只指向 sessions root 内已封存的段。fork 子会话 须引用父段并写 fork 边，不得复制父条目或重写父条目 id；子会话头 须保留 parent_session 与 fork_at_entry_id。
