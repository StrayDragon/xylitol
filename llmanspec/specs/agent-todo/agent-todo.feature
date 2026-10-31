# language: zh-CN
# purpose: 会话内 Todo SSOT — Custom latest-wins 持久、内置两工具（rewrite/update）、出站 AgentStatusBar 以 <todo> 子树投影当前清单、status 三态、in_progress 可多条、content ≤80 标量；类型化 live 事件与 TUI 下缘待办栏（三区可折、宽屏三栏）；不 persist 栏消息、无注册表。

功能: agent-todo

  @req:r1118
  规则: todo-domain-model
    会话 Todo 列表 须为有序条目集合；每条含稳定 id、非空 content（trim 后 Unicode 标量 ≤ 80）、status ∈ pending|in_progress|completed。空表合法。completed 须保留在表内直至整表 rewrite 不再包含该 id。省略 id 写入时 须由服务端铸造持久短 id（`t_` 前缀），不得按条目序号生成 `todo-{idx}`。读取旧 `agent_todo` 快照时，status 为 cancelled 的行 须丢掉且 不得使整表反序列化失败。本波 不得要求 max_attempt、assignee、namespace、cancelled 状态或 plan 类字段。

    场景: 待办严格模型形状
      当 读取 Todo 领域模型形状
      那么 条目为有序集合且 content 与状态受约束
  @req:r1123
  规则: custom-agent-todo-latest-wins
    Todo 真源 须以会话 Custom 条目持久化：custom_type 为 agent_todo，载荷为当前全量 items 快照。每次成功变更 须 append 一条完整快照；读取 须在当前 leaf 分支上 latest-wins（全叶扫描，不得仅依赖裁切后的 LLM context 窗）。不得另开旁路文件当真源；不得用会进模型前缀的 CustomMessage/session_env 冒充 Todo SSOT。

    场景: 待办快照最新胜出
      假如 已绑定会话的 todo 网关
      当 todo_rewrite 先写 A 表再覆盖为 B 表
      那么 读取当前清单为最新快照
  @req:r1124
  规则: todo-request-time-prefix-inject
    agent_todo Custom 快照 不得作为会话消息折叠进 provider 输入前缀（含 system / session_env）；SSOT 仍只在 Custom latest-wins。每次出站 generate，在历史折叠之后，须经 AgentStatusBar 投影：若当前 leaf 上 latest 清单非空，须在投影末尾追加恰好一条 user 行，根标签为 `<agent_status_bar>`，清单为子树 `<todo>`，每条目为带 id 与 status 属性的 `<item>`、正文为转义后的 content；空表 须省略该行（不得只发空根或空 `<todo/>`）。该行 不得写入会话 transcript / JSONL，不得进入系统提示前缀，不得并入 session_env。token 估计 须与真实请求同形计入该行；compact 摘要请求 不得投影状态栏。产品 不得再向模型暴露 todo_list 工具。由单测覆盖；不得为静态存在性单独扩 BDD step。

    场景: 待办快照之外前缀
      当 读取 Todo 快照投影边界
      那么 Custom 快照不进 provider 前缀且 SSOT 唯一
  @req:r1125
  规则: todo-tools-semantics
    产品默认工具表（Print 与 TUI 共用 builtins 基座）须提供且仅提供两 Todo 工具：todo_rewrite（整表替换，成功返回当前全表 `{items}`，空表清除）、todo_update（参数为 `items` 数组，每条 须有 id，且至少改 status、content 或 after_id 之一；成功返回当前全表 `{items}`）。未知 id 或一条没有任何改动字段 须拒绝整批写入并返回可读错误。成功写入后 须与 latest agent_todo 快照一致。todo_update 须提供非空 prompt_guidelines，标明 rewrite 用于建表/推翻、update 用于按 id 批量勾进度，并说明非空清单出现在 `<agent_status_bar>` 的 `<todo>` 子树。本波 不得向模型暴露 todo_list，不得要求 filter 参数，不得另增 todo_add / todo_remove 工具名。

    场景: 待办工具写入与拒绝
      假如 已绑定会话的 todo 网关
      当 todo_rewrite 写入两 in_progress 条目
      那么 写入成功且返回全表
      当 todo_update 更新未知 id
      那么 整批写入被拒绝且返回可读错误
  @req:r1126
  规则: multiple-in-progress-allowed
    in_progress 条目可为 0 或多条。todo_rewrite / todo_update 不得因存在两条及以上 in_progress 拒绝写入。待办栏 doing 列 须按 SSOT 表序展示全部 in_progress。

    场景: 多个于进行中已接受
      假如 已绑定会话的 todo 网关
      当 todo_rewrite 写入两 in_progress 条目
      那么 写入成功且返回全表
  @req:r1127
  规则: todo-tool-concurrency-barrier
    todo_rewrite / todo_update 须为 Barrier（或等价 Sequential）并发类，与其它会改会话状态的内置工具同族；调度 须尊重该类。分类表由单测覆盖，不得为静态表单独扩 BDD step。

    场景: 待办并发屏障
      当 读取 todo 工具调度分类
      那么 todo_rewrite 与 todo_update 为 Barrier 并发类
  @req:r1128
  规则: status-bar-read-only-boundary
    Todo SSOT 为本波唯一业务真源。待办栏与 AgentStatusBar 若投影清单或完成数等摘要，须只读自本 SSOT；不得以栏 auto 摘要写回 Todo。本波模型可见清单走 r1124 出站 AgentStatusBar（`<todo>` 子树）。不得 persist 栏消息、不得做 Agent 列 publish / statusline_refresh / 读数注册表。禁止 todo_* 只写栏不写 SSOT。TUI 待办栏不是 TUI 状态条，也不是 AgentStatusBar 投影行。

    场景: 状态栏读取仅边界
      当 读取 Todo SSOT 只读边界
      那么 待办栏摘要只读自 SSOT
  @req:r1129
  规则: tui-todo-bar-summary
    产品 TUI 在有 Todo 条目时 必须以下缘固定区的待办栏呈现同源列表，不得以对话条目充当主清单。待办栏 必须默认收起为单行摘要，形如 `▸ 待办 2/5 · <当前 doing 正文>`：`2/5` 为已完成数与总数，`· <当前 doing 正文>` 只在存在 in_progress 条目时显示，多条 in_progress 时 必须取 SSOT 表序最前一条。点击摘要行 必须展开为扁平列表，再次点击 必须收起；展开状态只在本会话内有效，不得跨进程或跨会话保留。展开态 必须按状态分组排列 doing → pending → completed，组内顺序 必须与 SSOT 表序一致，不得为展示重排真源。待办栏 不得画上下轮廓，不得画分组翼头，不得渲染可点的分组折叠三角。空表 必须整栏消失并占 0 行。待办栏正文 必须可应用内划选并复制。不得以常驻 Plan/双栏侧栏、status/footer/toast notice 或 ScrollNotice 刷墙作为主清单。本波 不得要求用户手改条目或 /todo slash。

    场景: 待办栏默认只占一行摘要
      当 以场景构建器回放含一条 in_progress 与前后翼条目的 TodoUpdated
      那么 待办栏只有一行摘要且含完成数与总数
      并且 待办栏 不画上下轮廓且 不出现分组翼头

    场景: 点击摘要行展开为分组列表
      当 以场景构建器回放含一条 in_progress 与前后翼条目的 TodoUpdated
      并且 点击待办栏摘要行
      那么 待办栏按 doing → pending → completed 分组展开
      并且 组内顺序与 SSOT 表序一致
  @req:r3051
  规则: tui-todo-bar-status-glyphs
    待办栏 必须以三档前导字符与亮度表达状态，不得使用 `[ ]` / `[~]` / `[x]` 勾选字形。进行中条目 必须用 `→` 与 `on_surface` 正文加粗；未完成条目 必须用 `·` 与 `muted` 正文；已完成条目 必须用 `✓` 与 `tool` 正文并叠加 dim。三档取色 必须只经语义 token 派生，浅色与深色主题 均适用。ASCII 字形档 必须分别回落 `>` / `.` / `x`。

    场景: 状态字符与亮度三档
      当 以场景构建器回放含一条 in_progress 与前后翼条目的 TodoUpdated 并点击待办栏摘要行
      那么 进行中条目以箭头与最亮正文呈现
      并且 未完成条目以中点与常规 muted 呈现
      并且 已完成条目以勾号与最暗 dim 呈现
  @req:r3052
  规则: tui-todo-bar-overflow
    待办栏可见行 必须至多 min(6, ⌊term_rows/4⌋) 且至少 1 行，行数 必须计入 Fixed-Zone Footprint。条目超出可见行时 必须在末行以 `… 还有 N 项 · 滚轮查看` 提示剩余项数，并在栏内用指针滚轮滚动，不得在边框上写 `↑ N more` / `↓ N more`，不得用省略号吃正文。条目绘制 必须按栏宽换行（优先空格；CJK 按字）；续行悬挂 4 cell 与前导字符对齐。

    场景: 超长清单在栏内滚动
      当 以场景构建器回放七条待办并点击待办栏摘要行
      那么 待办栏至多六行且末行提示剩余项数
  @req:r1130
  规则: tui-resume-and-tool-refresh
    TUI 在 resume / 切换会话后 须自当前 leaf 上 latest agent_todo 快照重建待办栏（默认摘要与展开形态同 r1129）；todo_* 工具成功结束后 须经类型化 TodoUpdated 刷新为与 SSOT 一致。Print 面 不得要求待办栏 UI，但工具与持久 须仍可用（同源 SSOT）。

    场景: 待办栏续跑匹配在线
      当 以场景构建器回放 todo_rewrite 成功调用的直播与 travel 重建
      那么 两种路径的待办栏摘要 一致且同源自 SSOT
      并且 todo_* 工具块 body 仍为清单行形态
  @req:r1119
  规则: compact-preserves-todo-snapshot
    压缩或裁切会话上下文时，若操作会丢掉当前 leaf 上全部 agent_todo 快照，System 须在裁切后重新 append 最新全量快照，使后续 TUI / resume / 出站 AgentStatusBar 仍可读到同一逻辑列表。不得静默永久丢弃用户 Todo。

    场景: 压缩再追加待办快照
      假如 已绑定会话的 todo 网关
      当 执行压后 todo 保全
      那么 最新快照被重追加到会话
  @req:r1120
  规则: todo-builtins-first-turn-freeze
    todo_rewrite / todo_update 属核心 builtins，须在会话首次工具表定稿前进入可见工具表；不得在回合中途热加这两个名字。与既有首轮冻表门闸一致；由单测覆盖，不得单独扩 BDD step。

    场景: 待办内置首次回合冻结
      当 读取首回合工具定稿清单
      那么 todo builtins 在定稿前进入可见工具表
  @req:r1121
  规则: export-shows-agent-todo
    会话导出（HTML/JSONL 或等价）须能呈现 agent_todo Custom 快照（或等价标记），不得将其伪装成用户消息。

    场景: 导出包含agent待办
      假如 已绑定会话的 todo 网关
      当 todo_rewrite 写入两 in_progress 条目
      那么 导出条目含 agent_todo 自定义记录而非用户消息
  @req:r1842
  规则: todo-content-max-length
    todo_rewrite / todo_update 写入的 content trim 后 Unicode 标量 须 ≤ 80；超长 须拒绝写入并返回可读错误，不得截断后入库。读取已持久的超长 agent_todo 快照 须仍可按栏宽换行绘制，不得因超长丢表。

    场景: 待办内容越过80拒绝
      假如 已绑定会话的 todo 网关
      当 写入超过 80 标量的 content
      那么 超长写入被拒绝且原表不变
  @req:r1122
  规则: typed-live-projection-event
    todo_* 成功写入后 host 须经领域事件发布类型化 TodoList 全量快照（空表亦然，语义为清除）；产品 client 的 live 待办栏投影 须源自该事件，不得依赖端侧解析工具结果字符串或调用 args 维持待办栏。resume / 重建 须仍读 agent_todo SSOT 快照，与 live 投影 latest-wins 同构。事件未送达（如旧线协议端）时待办栏 须可降级为仅 resume 刷新，不得 panic。不得要求 LLM 前缀、SSOT 持久形态或工具结果 JSON 形态为此改变。

    场景: 待办更新发出类型化事件
      当 以触发 todo_update 工具的回合收集事件
      那么 回合事件流含 TodoUpdated 全量快照
