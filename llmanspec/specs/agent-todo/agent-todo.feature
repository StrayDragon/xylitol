# language: zh-CN
# capability: agent-todo
# purpose: 会话内 Todo SSOT — Custom latest-wins 持久、内置两工具（rewrite/update）、出站 AgentStatusBar 以 <todo> 子树投影当前清单、status 三态、in_progress 可多条、content ≤80 标量；类型化 live 事件与 TUI 下缘待办栏（三区可折、宽屏三栏）；不 persist 栏消息、无注册表。
# scope: src/protocol/, src/agent/, src/infra/tools/, src/app/tui/

功能: agent-todo

  @req:r1118
  规则: todo-domain-model
    会话 Todo 列表 须为有序条目集合；每条含稳定 id、非空 content（trim 后 Unicode 标量 ≤ 80）、status ∈ pending|in_progress|completed。空表合法。completed 须保留在表内直至整表 rewrite 不再包含该 id。省略 id 写入时 须由服务端铸造持久短 id（`t_` 前缀），不得按条目序号生成 `todo-{idx}`。读取旧 `agent_todo` 快照时，status 为 cancelled 的行 须丢掉且 不得使整表反序列化失败。本波 不得要求 max_attempt、assignee、namespace、cancelled 状态或 plan 类字段。
    # verified-by: src/infra/tools/todo.rs

    场景: todo-strict-model-shape
      当 读取 Todo 领域模型形状
      那么 条目为有序集合且 content 与状态受约束
  @req:r1123
  规则: custom-agent-todo-latest-wins
    Todo 真源 须以会话 Custom 条目持久化：custom_type 为 agent_todo，载荷为当前全量 items 快照。每次成功变更 须 append 一条完整快照；读取 须在当前 leaf 分支上 latest-wins（全叶扫描，不得仅依赖裁切后的 LLM context 窗）。不得另开旁路文件当真源；不得用会进模型前缀的 CustomMessage/session_env 冒充 Todo SSOT。

    场景: todo-snapshot-latest-wins
      假如 已绑定会话的 todo 网关
      当 todo_rewrite 先写 A 表再覆盖为 B 表
      那么 读取当前清单为最新快照
  @req:r1124
  规则: todo-request-time-prefix-inject
    agent_todo Custom 快照 不得作为会话消息折叠进 provider 输入前缀（含 system / session_env）；SSOT 仍只在 Custom latest-wins。每次出站 generate，在历史折叠之后，须经 AgentStatusBar 投影：若当前 leaf 上 latest 清单非空，须在投影末尾追加恰好一条 user 行，根标签为 `<agent_status_bar>`，清单为子树 `<todo>`，每条目为带 id 与 status 属性的 `<item>`、正文为转义后的 content；空表 须省略该行（不得只发空根或空 `<todo/>`）。该行 不得写入会话 transcript / JSONL，不得进入系统提示前缀，不得并入 session_env。token 估计 须与真实请求同形计入该行；compact 摘要请求 不得投影状态栏。产品 不得再向模型暴露 todo_list 工具。由单测覆盖；不得为静态存在性单独扩 BDD step。
    # verified-by: src/agent/prompt/status_bar.rs

    场景: todo-snapshot-outside-prefix
      当 读取 Todo 快照投影边界
      那么 Custom 快照不进 provider 前缀且 SSOT 唯一
  @req:r1125
  规则: todo-tools-semantics
    产品默认工具表（Print 与 TUI 共用 builtins 基座）须提供且仅提供两 Todo 工具：todo_rewrite（整表替换，成功返回当前全表 `{items}`，空表清除）、todo_update（参数为 `items` 数组，每条 须有 id，且至少改 status、content 或 after_id 之一；成功返回当前全表 `{items}`）。未知 id 或一条没有任何改动字段 须拒绝整批写入并返回可读错误。成功写入后 须与 latest agent_todo 快照一致。todo_update 须提供非空 prompt_guidelines，标明 rewrite 用于建表/推翻、update 用于按 id 批量勾进度，并说明非空清单出现在 `<agent_status_bar>` 的 `<todo>` 子树。本波 不得向模型暴露 todo_list，不得要求 filter 参数，不得另增 todo_add / todo_remove 工具名。

    场景: todo-tools-write-and-reject
      假如 已绑定会话的 todo 网关
      当 todo_rewrite 写入两 in_progress 条目
      那么 写入成功且返回全表
      当 todo_update 更新未知 id
      那么 整批写入被拒绝且返回可读错误
  @req:r1126
  规则: multiple-in-progress-allowed
    in_progress 条目可为 0 或多条。todo_rewrite / todo_update 不得因存在两条及以上 in_progress 拒绝写入。待办栏 doing 列 须按 SSOT 表序展示全部 in_progress。

    场景: multiple-in-progress-accepted
      假如 已绑定会话的 todo 网关
      当 todo_rewrite 写入两 in_progress 条目
      那么 写入成功且返回全表
  @req:r1127
  规则: todo-tool-concurrency-barrier
    todo_rewrite / todo_update 须为 Barrier（或等价 Sequential）并发类，与其它会改会话状态的内置工具同族；调度 须尊重该类。分类表由单测覆盖，不得为静态表单独扩 BDD step。
    # verified-by: src/agent/tools/mod.rs

    场景: todo-concurrency-barrier
      当 读取 todo 工具调度分类
      那么 todo_rewrite 与 todo_update 为 Barrier 并发类
  @req:r1128
  规则: status-bar-read-only-boundary
    Todo SSOT 为本波唯一业务真源。待办栏与 AgentStatusBar 若投影清单或完成数等摘要，须只读自本 SSOT；不得以栏 auto 摘要写回 Todo。本波模型可见清单走 r1124 出站 AgentStatusBar（`<todo>` 子树）。不得 persist 栏消息、不得做 Agent 列 publish / statusline_refresh / 读数注册表。禁止 todo_* 只写栏不写 SSOT。TUI 待办栏不是 TUI 状态条，也不是 AgentStatusBar 投影行。
    # verified-by: src/agent/prompt/status_bar.rs

    场景: status-bar-read-only-boundary
      当 读取 Todo SSOT 只读边界
      那么 待办栏摘要只读自 SSOT
  @req:r1129
  规则: tui-checklist-collapsible
    产品 TUI 在有 Todo 条目时 须以下缘固定区待办栏呈现同源列表，不得以对话条目充当主清单。待办栏 须分三区：当前 doing（全部 in_progress）、后翼 pending、前翼 completed；区内顺序 须与 SSOT 表序一致，不得为展示重排真源。有条目时默认 须展开三区只读列表，三列 须可独立折叠。无 in_progress 时 须展开非空翼条目，不得假装一条当前任务。计数 须写在翼头（形如 `▾ n doing` / `▾ n pending` / `▾ n completed`，折上为 `▸`），零计数翼 须仍画翼头（形如 `▾ 0 doing` / `▾ 0 pending` / `▾ 0 completed`），不得假装条目；不得另画 done/todo 行，不得仅用无标签的 N/M。doing 条目 须用正常 on-surface 正文，不得用 `[~]` 勾选字形。待办栏正文 须可应用内划选并复制（翼头三角列除外，点三角仍折）。终端列宽达到 DESIGN spacing.diff-side-by-side-min-cols（100）时 须将三区分为栏（doing|pending|completed，顶对齐，高度取最高栏）。三列槽位 须始终三等分（余数列给末栏）；折叠 须只收起该列正文，不得改变列宽或列起点，不得让邻列条目因折上而换行重排。更窄 须竖叠（doing、pending、completed 自上而下）。条目绘制 须按当前栏 cell 宽换行（优先空格；CJK 按字）；pending/completed 续行悬挂 4 cell 与勾选正文对齐；不得省略截断。待办栏内容可见行 须 ≤ min(6, ⌊term_rows/4⌋) 且至少 2；超出 须在栏内滚动（指针在栏上滚轮），翼头计数仍为全表；不得省略正文。有表时 须画上下 `─` 轮廓（与 editor 操作区同形）；未滚入视口的内容 须在对应边框写入 `↑ N more` / `↓ N more`（N 为未显示行数），不得用省略号或替换末字吃正文。空表 须整栏消失且 不得画轮廓。轮廓行计入 Fixed-Zone Footprint，不计入内容行上限。用户展开某一翼 须见该翼完整只读勾选列表（可经栏内滚动）。不得以常驻 Plan/双栏侧栏、status/footer/toast notice 或 ScrollNotice 刷墙作为主清单。本波 不得要求用户手改条目或 /todo slash。

    场景: todo-bar-default-shows-in-progress
      当 以场景构建器回放含一条 in_progress 与前后翼条目的 TodoUpdated
      那么 待办栏默认展开当前任务与两翼条目
      并且 对话条目中 不出现 checklist 投影行
      并且 翼头 分别标注 completed 与 pending 且 不出现 done/todo 行或无标签 N/M
  @req:r1130
  规则: tui-resume-and-tool-refresh
    TUI 在 resume / 切换会话后 须自当前 leaf 上 latest agent_todo 快照重建待办栏（默认露出与三区折叠同 r1129）；todo_* 工具成功结束后 须经类型化 TodoUpdated 刷新为与 SSOT 一致。Print 面 不得要求待办栏 UI，但工具与持久 须仍可用（同源 SSOT）。

    场景: todo-bar-resume-matches-live
      当 以场景构建器回放 todo_rewrite 成功调用的直播与 travel 重建
      那么 两种路径的待办栏默认露出 一致且同源自 SSOT
      并且 todo_* 工具块 body 仍为清单行形态
  @req:r1119
  规则: compact-preserves-todo-snapshot
    压缩或裁切会话上下文时，若操作会丢掉当前 leaf 上全部 agent_todo 快照，System 须在裁切后重新 append 最新全量快照，使后续 TUI / resume / 出站 AgentStatusBar 仍可读到同一逻辑列表。不得静默永久丢弃用户 Todo。

    场景: compaction-reappends-todo-snapshot
      假如 已绑定会话的 todo 网关
      当 执行压后 todo 保全
      那么 最新快照被重追加到会话
  @req:r1120
  规则: todo-builtins-first-turn-freeze
    todo_rewrite / todo_update 属核心 builtins，须在会话首次工具表定稿前进入可见工具表；不得在回合中途热加这两个名字。与既有首轮冻表门闸一致；由单测覆盖，不得单独扩 BDD step。
    # verified-by: src/agent/tools/freeze.rs

    场景: todo-builtins-first-turn-freeze
      当 读取首回合工具定稿清单
      那么 todo builtins 在定稿前进入可见工具表
  @req:r1121
  规则: export-shows-agent-todo
    会话导出（HTML/JSONL 或等价）须能呈现 agent_todo Custom 快照（或等价标记），不得将其伪装成用户消息。

    场景: export-includes-agent-todo
      假如 已绑定会话的 todo 网关
      当 todo_rewrite 写入两 in_progress 条目
      那么 导出条目含 agent_todo 自定义记录而非用户消息
  @req:r1842
  规则: todo-content-max-length
    todo_rewrite / todo_update 写入的 content trim 后 Unicode 标量 须 ≤ 80；超长 须拒绝写入并返回可读错误，不得截断后入库。读取已持久的超长 agent_todo 快照 须仍可按栏宽换行绘制，不得因超长丢表。

    场景: todo-content-over-80-rejected
      假如 已绑定会话的 todo 网关
      当 写入超过 80 标量的 content
      那么 超长写入被拒绝且原表不变
  @req:r1122
  规则: typed-live-projection-event
    todo_* 成功写入后 host 须经领域事件发布类型化 TodoList 全量快照（空表亦然，语义为清除）；产品 client 的 live 待办栏投影 须源自该事件，不得依赖端侧解析工具结果字符串或调用 args 维持待办栏。resume / 重建 须仍读 agent_todo SSOT 快照，与 live 投影 latest-wins 同构。事件未送达（如旧线协议端）时待办栏 须可降级为仅 resume 刷新，不得 panic。不得要求 LLM 前缀、SSOT 持久形态或工具结果 JSON 形态为此改变。
# re-review(c2826): 复审结论——本 capability 管辖行为不变；分支内改动仅测试基建与可见性再导出（2026-09-28）

# re-review(c2827): 复审结论——本 capability 管辖行为不变；分支内改动为 BDD 场景落地、BDD 测试基建（steps/bindings/驱动旋钮与探针）与可见性再导出（2026-09-28）

    场景: todo-update-emits-typed-event
      当 以触发 todo_update 工具的回合收集事件
      那么 回合事件流含 TodoUpdated 全量快照
# re-review(c2835): 复审结论——本 capability 管辖行为不变；仅协议载体常量与死变体清理。（2026-09-29）

# re-review(c2837): c2837 编译隔离变更影响本 scope——agent/infra 公开化与 BDD 测试辅助面收敛（纯可见性扩张与测试基建，无行为变化）。场景映射不变量保持；已复核。（2026-10-06）

# re-review(c2838): c2838 intra-doc 链接治理触及本 scope 内源码 doc 注释（纯文档、无行为变化）。场景映射不变量保持；已复核。（2026-10-06）
# re-review(c2853): 承载分支 sdd/2026-10-review-fixes 触及本 scope（BDD 步骤卫生 / 既有 codec·host 改动）；本 capability 管辖行为不变。场景映射不变量保持。（2026-10-09）
# re-review(c2854): 复审结论——本 capability 管辖行为不变；分支改动为 Host 产品入口收口与 BDD 词表，未改本规则可观察语义。（2026-10-09）
# re-review(c2855): 复审结论——本 capability 管辖行为不变；分支改动为 Api 优先启发式 /3、下线本地词表与客户端通知栈，未改本规则可观察语义。（2026-10-10）
# re-review(c2865): 复审结论——本 capability 管辖行为不变；分支改动为流式半截闭合围栏修剪（引擎 Markdown + 产品 streaming_assistant 高度），未改本规则可观察语义。（2026-10-10）
