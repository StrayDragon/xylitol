# language: zh-CN
# capability: agent-todo
# purpose: 会话内 Todo SSOT — Custom latest-wins 持久、内置三工具读写、in_progress 可多条、content ≤80 标量、不进 LLM 前缀；类型化 live 事件与 TUI 下缘待办栏（三区可折、宽屏三栏）、状态栏只读投影边界。
# scope: src/protocol/, src/agent/, src/infra/tools/, src/app/tui/

功能: agent-todo

  @req:r1118 @human
  场景: todo-domain-model
    - 会话 Todo 列表 MUST 为有序条目集合；每条含稳定 id、非空 content（trim 后 Unicode 标量 ≤ 80）、status ∈ pending|in_progress|completed|cancelled。空表合法。completed/cancelled MUST 保留在表内直至整表 rewrite 删除。本波 MUST NOT 要求 max_attempt、assignee、namespace 或 plan 类字段。

  @req:r1123 @human
  场景: custom-agent-todo-latest-wins
    - Todo 真源 MUST 以会话 Custom 条目持久化：custom_type 为 agent_todo，载荷为当前全量 items 快照。每次成功变更 MUST append 一条完整快照；读取 MUST 在当前 leaf 分支上 latest-wins（全叶扫描，MUST NOT 仅依赖裁切后的 LLM context 窗）。MUST NOT 另开旁路文件当真源；MUST NOT 用会进模型前缀的 CustomMessage/session_env 冒充 Todo SSOT。

  @req:r1124 @human
  场景: todo-not-in-llm-prefix
    - agent_todo Custom 快照 MUST NOT 经会话→模型投影进入 provider 输入前缀（含 system / session_env / 状态栏族标签）。模型可见 Todo MUST 仅来自工具结果或模型主动调用 todo_list。由单测覆盖；MUST NOT 为静态存在性单独扩 BDD step。

  @req:r1125 @human
  场景: todo-tools-semantics
    - 产品默认工具表（Print 与 TUI 共用 builtins 基座）MUST 提供三工具：todo_list（只读返回当前全表）、todo_rewrite（整表替换后返回全表）、todo_update（按 id 改 status 与/或 content，至少改一项；成功返回全表）。未知 id MUST 拒绝并返回可读错误。成功写入后 MUST 与 atd2 快照一致。本波 MUST NOT 要求 filter 参数。

  @req:r1126 @human
  场景: multiple-in-progress-allowed
    - in_progress 条目可为 0 或多条。todo_rewrite / todo_update MUST NOT 因存在两条及以上 in_progress 拒绝写入。待办栏 doing 列 MUST 按 SSOT 表序展示全部 in_progress。

  @req:r1127 @human
  场景: todo-tool-concurrency-barrier
    - todo_list / todo_rewrite / todo_update MUST 为 Barrier（或等价 Sequential）并发类，与其它会改会话状态的内置工具同族；调度 MUST 尊重该类。分类表由单测覆盖，MUST NOT 为静态表单独扩 BDD step。

  @req:r1128 @human
  场景: status-bar-read-only-boundary
    - Todo SSOT 为本波唯一业务真源。待办栏与日后状态栏/Agent 列若投影完成数等摘要，MUST 只读自本 SSOT；MUST NOT 以栏 auto 摘要写回 Todo；MUST NOT 要求本波实现栏注入、Agent 列 publish 或 refresh 工具。禁止 todo_* 只写栏不写 SSOT。待办栏不是 status。

  @req:r1129 @human
  场景: tui-checklist-collapsible
    - 产品 TUI 在有 Todo 条目时 MUST 以下缘固定区待办栏呈现同源列表，MUST NOT 以对话条目充当主清单。待办栏 MUST 分三区：当前 doing（全部 in_progress）、后翼 pending、前翼（completed 与 cancelled）；区内顺序 MUST 与 SSOT 表序一致，MUST NOT 为展示重排真源。有条目时默认 MUST 展开三区只读列表，三列 MUST 可独立折叠。无 in_progress 时 MUST 展开非空翼条目，MUST NOT 假装一条当前任务。计数 MUST 写在翼头（形如 `▾ n doing` / `▾ n pending` / `▾ n completed`，折上为 `▸`），零计数翼 MUST 仍画翼头（形如 `▾ 0 doing` / `▾ 0 pending` / `▾ 0 completed`），MUST NOT 假装条目；MUST NOT 另画 done/todo 行，MUST NOT 仅用无标签的 N/M。doing 条目 MUST 用正常 on-surface 正文，MUST NOT 用 `[~]` 勾选字形。待办栏正文 MUST 可应用内划选并复制（翼头三角列除外，点三角仍折）。终端列宽达到 DESIGN spacing.diff-side-by-side-min-cols（100）时 MUST 将三区分为栏（doing|pending|completed，顶对齐，高度取最高栏）。三列槽位 MUST 始终三等分（余数列给末栏）；折叠 MUST 只收起该列正文，MUST NOT 改变列宽或列起点，MUST NOT 让邻列条目因折上而换行重排。更窄 MUST 竖叠（doing、pending、completed 自上而下）。条目绘制 MUST 按当前栏 cell 宽换行（优先空格；CJK 按字）；pending/completed 续行悬挂 4 cell 与勾选正文对齐；MUST NOT 省略截断。待办栏内容可见行 MUST ≤ min(6, ⌊term_rows/4⌋) 且至少 2；超出 MUST 在栏内滚动（指针在栏上滚轮），翼头计数仍为全表；MUST NOT 省略正文。有表时 MUST 画上下 `─` 轮廓（与 editor 操作区同形）；未滚入视口的内容 MUST 在对应边框写入 `↑ N more` / `↓ N more`（N 为未显示行数），MUST NOT 用省略号或替换末字吃正文。空表 MUST 整栏消失且 MUST NOT 画轮廓。轮廓行计入 Fixed-Zone Footprint，不计入内容行上限。用户展开某一翼 MUST 见该翼完整只读勾选列表（可经栏内滚动）。MUST NOT 以常驻 Plan/双栏侧栏、status/footer/toast notice 或 ScrollNotice 刷墙作为主清单。本波 MUST NOT 要求用户手改条目或 /todo slash。

  @req:r1130 @human
  场景: tui-resume-and-tool-refresh
    - TUI 在 resume / 切换会话后 MUST 自当前 leaf 上 latest agent_todo 快照重建待办栏（默认露出与三区折叠同 r1129）；todo_* 工具成功结束后 MUST 经类型化 TodoUpdated 刷新为与 SSOT 一致。Print 面 MUST NOT 要求待办栏 UI，但工具与持久 MUST 仍可用（同源 SSOT）。

  @req:r1119 @human
  场景: compact-preserves-todo-snapshot
    - 压缩或裁切会话上下文时，若操作会丢掉当前 leaf 上全部 agent_todo 快照，System MUST 在裁切后重新 append 最新全量快照，使后续 todo_list / TUI / resume 仍可读到同一逻辑列表。MUST NOT 静默永久丢弃用户 Todo。

  @req:r1120 @human
  场景: todo-builtins-first-turn-freeze
    - todo_list / todo_rewrite / todo_update 属核心 builtins，MUST 在会话首次工具表定稿前进入可见工具表；MUST NOT 在回合中途热加这三个名字。与既有首轮冻表门闸一致；由单测覆盖，MUST NOT 单独扩 BDD step。

  @req:r1121 @human
  场景: export-shows-agent-todo
    - 会话导出（HTML/JSONL 或等价）MUST 能呈现 agent_todo Custom 快照（或等价标记），MUST NOT 将其伪装成用户消息。

  @req:r1842 @human
  场景: todo-content-max-length
    - todo_rewrite / todo_update 写入的 content trim 后 Unicode 标量 MUST ≤ 80；超长 MUST 拒绝写入并返回可读错误，MUST NOT 截断后入库。读取已持久的超长 agent_todo 快照 MUST 仍可按栏宽换行绘制，MUST NOT 因超长丢表。

  @req:r1122 @human
  场景: typed-live-projection-event
    - todo_* 成功写入后 host MUST 经领域事件发布类型化 TodoList 全量快照（空表亦然，语义为清除）；产品 client 的 live 待办栏投影 MUST 源自该事件，MUST NOT 依赖端侧解析工具结果字符串或调用 args 维持待办栏。resume / 重建 MUST 仍读 agent_todo SSOT 快照，与 live 投影 latest-wins 同构。事件未送达（如旧线协议端）时待办栏 MUST 可降级为仅 resume 刷新，MUST NOT panic。MUST NOT 要求 LLM 前缀、SSOT 持久形态或工具结果 JSON 形态为此改变。

  @executable @req:r1129
  场景: todo-bar-default-shows-in-progress
    当 以场景构建器回放含一条 in_progress 与前后翼条目的 TodoUpdated
    那么 待办栏默认展开当前任务与两翼条目
    并且 对话条目中 MUST NOT 出现 checklist 投影行
    并且 翼头 MUST 分别标注 completed 与 pending 且 MUST NOT 出现 done/todo 行或无标签 N/M

  @executable @req:r1130
  场景: todo-bar-resume-matches-live
    当 以场景构建器回放 todo_rewrite 成功调用的直播与 travel 重建
    那么 两种路径的待办栏默认露出 MUST 一致且同源自 SSOT
    并且 todo_* 工具块 body MUST 仍为清单行形态
