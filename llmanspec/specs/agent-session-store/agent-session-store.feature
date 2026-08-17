# language: zh-CN
# migrated from tests/features/session.feature
功能: agent-session-store
  @req:s1
  场景: create-load
    假定 会话存储目录已初始化
    当 创建一个新会话 "test-session"
    并且 向会话追加一条消息 "用户问好"
    并且 加载会话 "test-session"
    那么 会话包含 1 条记录
    并且 记录类型为 "message"

  场景: list-sessions
    假定 存在会话 "session-a"
    并且 存在会话 "session-b"
    当 列出所有会话
    那么 结果包含 "session-a"
    并且 结果包含 "session-b"

  @req:s5
  场景: fork
    假定 存在会话 "parent" 包含 10 条记录
    当 在记录 5 处分叉创建会话 "child"
    那么 会话 "child" 包含 5 条记录
    并且 会话 "child" 包含一个 branch_summary 记录

  场景: tree-nav
    假定 存在会话树: "root" → "branch-a" → "branch-b"
    当 导航到 "branch-b"
    那么 上下文包含 branch-a 和 branch-b 的摘要

  场景: model-change
    假定 存在会话 "model-test"
    当 将会话模型从 "gpt-4o" 切换为 "claude-sonnet"
    那么 会话包含 model_change 记录
    并且 model_change 记录显示 provider 为 "anthropic"

  场景: thinking-change
    假定 存在会话 "think-test"
    当 切换思考级别为 "high"
    那么 会话包含 thinking_level_change 记录

  场景: jsonl-format
    假定 存在会话 "format-test"
    当 向会话追加 3 条不同类型的记录
    那么 JSONL 文件每行是一个完整的 JSON 对象
    并且 第一行包含 version 字段

  场景: label-set
    假定 存在会话 "label-test"
    当 向会话追加一条消息 "标记我"
    并且 为最后一条记录设置标签 "重要"
    那么 该记录的标签为 "重要"

  场景: label-clear
    假定 存在会话 "label-clear-test"
    当 向会话追加一条消息 "标签测试"
    并且 为最后一条记录设置标签 "重要"
    并且 清除该记录的标签
    那么 该记录没有标签

  @req:r31
  场景: happy
    假如 存在快照
    当 以 snapshot_id 与新 prompt 调用 spawn
    那么 新 agent 实例以快照上下文启动

  @req:r41
  场景: happy
    假如 对话超过上下文窗口 75%
    当 触发 compaction
    那么 较早回合被摘要并替换为 compact system message

  @req:s2
  场景: types
    假如 append 各类型条目
    当 加载会话
    那么 各条目保留类型与数据

  @req:s3
  场景: list
    假如 存在多个会话
    当 调用 list()
    那么 返回全部 session id

  @req:s4
  场景: no-untagged-migration
    假如 磁盘 JSONL 含旧 untagged thinking 对象
    当 load 并解析为 AgentPart
    那么 不迁移为合法 Thinking；走拒绝或跳过

  @req:s6
  场景: branch-summary
    假如 切点前有 5 条条目
    当 调用 generateBranchSummary
    那么 产出汇总该 5 条的 CompactionEntry

  @req:s7
  场景: write-atomicity
    假如 存在既有会话文件
    当 重写路径（flush merge / header repair）执行后进程任意时点终止
    那么 盘上文件保持旧完整内容或新完整内容；无截断或半行混合；同一会话至多一个进程写入

  @req:s9
  场景: basic-fork
    假如 会话 A 有 e0 至 e9 共 10 条
    当 在 e4 fork
    那么 子会话有 5 条（e0..e4）加 branch_summary

  @req:s9
  场景: full-fork
    假如 会话 A 有 5 条
    当 在最后条目 e4 fork
    那么 子会话含全部 5 条且无 branch_summary

  @req:s9
  场景: parent-link
    假如 经 fork 创建子会话
    当 加载子会话 header
    那么 parent_session 字段匹配父 id

  @req:s10
  场景: summary
    假如 父会话有 3 条 user、7 条 assistant 与 5 次工具调用
    当 生成分支摘要
    那么 摘要含总条目数与工具调用数

  @req:s10
  场景: empty-summary
    假如 fork 点后无剩余条目
    当 生成分支摘要
    那么 摘要为空串或表明无跳过内容

  @req:s11
  场景: agent-fork
    假如 agent 会话活动
    当 调用 fork_session(e4)
    那么 返回新子 session id 且磁盘存在子会话文件

  @req:s16
  场景: cwd-ok
    假如 会话文件 cwd 为 /tmp 且目录存在
    当 调用 load
    那么 正常返回条目

  @req:s16
  场景: cwd-missing
    假如 会话文件 cwd 为 /nonexistent
    当 调用 load
    那么 错误引用缺失目录 /nonexistent

  @req:ex1
  场景: html
    假如 会话有消息
    当 调用 export_to_html
    那么 写入 HTML 文件

  @req:ex2
  场景: jsonl
    假如 会话有条目
    当 调用 export_to_jsonl
    那么 文件每行一个 JSON 对象

  @req:ex3
  场景: import
    假如 存在合法 JSONL 文件
    当 调用 import_from_jsonl
    那么 条目加载到新会话

  @req:ex4
  场景: html-tools
    假如 会话含工具调用
    当 打开导出 HTML
    那么 工具结果渲染为可读块

  @req:ex5
  场景: share-unconfigured
    假如 未配置 token 调用 share
    当 调用 share
    那么 返回配置指引消息

  @req:sc1
  场景: exists
    假如 存储 cwd 在磁盘存在
    当 调用 assert_session_cwd_exists
    那么 调用成功

  @req:sc1
  场景: missing
    假如 存储 cwd 在磁盘不存在
    当 调用 assert_session_cwd_exists
    那么 返回 MissingSessionCwdError

  @req:sc2
  场景: cwd-error
    假如 存储 cwd 不存在
    当 创建 MissingSessionCwdError
    那么 错误携带会话文件路径与两个 cwd

  @req:sc3
  场景: cwd-format
    假如 存在 MissingSessionCwdError
    当 调用 format_missing_session_cwd_error
    那么 返回含两个 cwd 的面向用户消息

  @req:sc4
  场景: cli-missing
    假如 CLI 启动时会话 cwd 缺失
    当 格式化并展示错误
    那么 引导用户至 fallback cwd

  @req:sp1
  场景: manager-impls-store
    假如 对照 SessionStore 检查 SessionManager
    当 检查 impl
    那么 实现 trait 且可编译

  @req:sp2
  场景: manager-impls-store
    假如 对照 SessionStore 测试 SessionManager
    当 检查 trait 边界
    那么 三方法可编译且往返数据

  @req:sp6
  场景: journal-reads-recent
    假如 会话有 50 个事件且 journal limit 为 10000
    当 调用 read_recent(s0, 10)
    那么 按 seq 顺序返回最后 10 个事件

  @req:sp3
  场景: std-export-io
    假如 组合根为导出构造 Agent
    当 注入 ExportIo
    那么 具体类型为 infra/ 的 StdExportIo

  @req:s12
  场景: defer-before-assistant
    假如 新 persisted session 仅有 user 消息已 append
    当 检查磁盘 jsonl
    那么 文件尚不存在或未含完整挂起条目刷出；首条 assistant append 后文件存在且含 user+assistant

  @req:s12
  场景: in-memory-no-file
    假如 SessionManager::in_memory()
    当 append 多条消息
    那么 无 session 文件被创建且 load 仍可读到条目
