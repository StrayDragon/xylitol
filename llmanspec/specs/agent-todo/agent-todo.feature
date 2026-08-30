# language: zh-CN
# capability: agent-todo
# purpose: 会话内 Todo SSOT — Custom latest-wins 持久、内置三工具读写、至多一条 in_progress、不进 LLM 前缀；TUI 可折 checklist 与状态栏只读投影边界。
# scope: src/protocol/, src/agent/, src/infra/tools/, src/app/tui/

功能: agent-todo

  @req:atd1 @human
  场景: todo-domain-model
    - 会话 Todo 列表 MUST 为有序条目集合；每条含稳定 id、非空 content（trim 后）、status ∈ pending|in_progress|completed|cancelled。空表合法。completed/cancelled MUST 保留在表内直至整表 rewrite 删除。本波 MUST NOT 要求 max_attempt、assignee、namespace 或 plan 类字段。

  @req:atd2 @human
  场景: custom-agent-todo-latest-wins
    - Todo 真源 MUST 以会话 Custom 条目持久化：custom_type 为 agent_todo，载荷为当前全量 items 快照。每次成功变更 MUST append 一条完整快照；读取 MUST 在当前 leaf 分支上 latest-wins（全叶扫描，MUST NOT 仅依赖裁切后的 LLM context 窗）。MUST NOT 另开旁路文件当真源；MUST NOT 用会进模型前缀的 CustomMessage/session_env 冒充 Todo SSOT。

  @req:atd3 @human
  场景: todo-not-in-llm-prefix
    - agent_todo Custom 快照 MUST NOT 经会话→模型投影进入 provider 输入前缀（含 system / session_env / 状态栏族标签）。模型可见 Todo MUST 仅来自工具结果或模型主动调用 todo_list。由单测覆盖；MUST NOT 为静态存在性单独扩 BDD step。

  @req:atd4 @human
  场景: todo-tools-semantics
    - 产品默认工具表（Print 与 TUI 共用 builtins 基座）MUST 提供三工具：todo_list（只读返回当前全表）、todo_rewrite（整表替换后返回全表）、todo_update（按 id 改 status 与/或 content，至少改一项；成功返回全表）。未知 id MUST 拒绝并返回可读错误。成功写入后 MUST 与 atd2 快照一致。本波 MUST NOT 要求 filter 参数。

  @req:atd5 @human
  场景: at-most-one-in-progress
    - 任一合法 Todo 快照中 in_progress 条目 MUST 至多一条。todo_rewrite / todo_update 若会导致两条及以上 in_progress，MUST 拒绝写入并返回可读错误，且 MUST NOT append 新快照。

  @req:atd6 @human
  场景: todo-tool-concurrency-barrier
    - todo_list / todo_rewrite / todo_update MUST 为 Barrier（或等价 Sequential）并发类，与其它会改会话状态的内置工具同族；调度 MUST 尊重该类。分类表由单测覆盖，MUST NOT 为静态表单独扩 BDD step。

  @req:atd7 @human
  场景: status-bar-read-only-boundary
    - Todo SSOT 为本波唯一业务真源。日后状态栏/Agent 列若投影完成数等摘要，MUST 只读自本 SSOT；MUST NOT 以栏 auto 摘要写回 Todo；MUST NOT 要求本波实现栏注入、Agent 列 publish 或 refresh 工具。禁止 todo_* 只写栏不写 SSOT。

  @req:atd8 @human
  场景: tui-checklist-collapsible
    - 产品 TUI 在有 Todo 条目时 MUST 以对话区可折叠 checklist 呈现同源列表：默认一行摘要（含完成/总数类计数，如 Todo · 2/5）；用户展开见完整只读勾选列表。MUST NOT 以常驻 Plan/双栏侧栏、status/footer/chrome toast 或 ScrollNotice 刷墙作为主清单。本波 MUST NOT 要求用户手改条目或 /todo slash。

  @req:atd9 @human
  场景: tui-resume-and-tool-refresh
    - TUI 在 resume / 切换会话后 MUST 自当前 leaf 上 latest agent_todo 快照重建 checklist；todo_* 工具成功结束后 MUST 刷新为与 SSOT 一致的摘要/列表。Print 面 MUST NOT 要求 checklist UI，但工具与持久 MUST 仍可用（同源 SSOT）。

  @req:atd10 @human
  场景: compact-preserves-todo-snapshot
    - 压缩或裁切会话上下文时，若操作会丢掉当前 leaf 上全部 agent_todo 快照，System MUST 在裁切后重新 append 最新全量快照，使后续 todo_list / TUI / resume 仍可读到同一逻辑列表。MUST NOT 静默永久丢弃用户 Todo。

  @req:atd11 @human
  场景: todo-builtins-first-turn-freeze
    - todo_list / todo_rewrite / todo_update 属核心 builtins，MUST 在会话首次工具表定稿前进入可见工具表；MUST NOT 在回合中途热加这三个名字。与既有首轮冻表门闸一致；由单测覆盖，MUST NOT 单独扩 BDD step。

  @req:atd12 @human
  场景: export-shows-agent-todo
    - 会话导出（HTML/JSONL 或等价）MUST 能呈现 agent_todo Custom 快照（或等价标记），MUST NOT 将其伪装成用户消息。
