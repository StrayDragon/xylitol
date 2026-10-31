# language: zh-CN
# purpose: 斜杠命令与 protocol::Command / dispatch 映射。

功能: app-tui-commands

  @req:r1188
  规则: slash-exit-model
    TUI 须解析斜杠命令 /exit（映射 Quit）与 /model：无参 idle 须打开替换 editor 槽的 fuzzy 模型列表（GetAvailableModels）；列表 须支持 ↑↓ 选模型行，可调模型 须支持 ←→ 与槽内 Shift+Tab 在焦点模型声明的 thinking_levels 上选档；宽内容区 须在焦点行铺开该模型声明档名（当前档以方括号标记），窄内容区 须仅显示单档标签；无思考或仅不可调 off 的模型 须显示 — 且方向键 不得改变状态。Enter 选定 须提交 (model, level)（可调默认配置列表末项，无思考为 off）经 SetModel/set_thinking_level 并关槽；有参 /model <id> 须直接 SetModel 并对可调模型默认末项、不可调为 off；键入 /model 参数时 MAY/须经包 SlashArgCompletionSource 内联补全（见 ati23）。busy 时无参 须仍可打开模型列表（BusySlashPolicy Allow；确认选定仍走既有 SetModel/thinking 与 NextTurn/下轮预告，见 ar25）；busy 有参直设 须更新 selected（NextTurn 生效，见 ar25）。不得再把无参 /model 做成 CycleModel；不得引入 /models 主路径；不得把未在该模型 thinking_levels 声明的档名当成可选项；未知斜杠 须显示系统错误行且 不得崩溃；idle Enter 提交以 / 开头的文本 须先经命令解析路径。

    场景: 斜杠退出请求退出
      当 以主机泵在 idle 提交 "/exit"
      那么 会话收到退出请求

    场景: 斜杠模型打开列表与未知错误
      当 以主机泵在 idle 提交 "/model"
      那么 模型列表槽打开且经驱动取可用模型
      当 关闭当前槽并提交 "/nope"
      那么 写入系统错误行且未退出且未崩溃

    场景: 斜杠模型带参数直接设置
      当 以主机泵在 idle 提交 "/model test-model"
      那么 有参直设经模型执行器更新且仅更新固定区
  @req:r1198
  规则: dispatch-shared
    除 Prompt、Quit 与传输专用命令外，斜杠执行 须走 app::core::dispatch 共享路径，不得发明平行命令词表。

    场景: 斜杠会话名称经共享分发
      当 以主机泵在 idle 提交 "/session-name 新名字"
      那么 会话名经共享 dispatch 写入一次
  @req:r1199
  规则: extensible-commands
    命令面 须允许后续追加斜杠（如 /compact /help）而不改引擎；MVP 仅要求 /exit 与 /model。

    场景: MVP需要模型与退出
      当 在产品命令模块解析 "/model" 与 "/exit"
      那么 分别得到模型与退出待发命令
  @req:r1201
  规则: debug-slash-retired
    产品 TUI 与 designing 组合 不得解析斜杠 /debug、/debug <scene>（空格）或 /debug:<scene>（冒号）。SlashCommandSource 不得列出 debug。提交上述文本 须按未知斜杠短提示且 不得崩溃、不得注入场景。场景装载见 app-tui-preview。

    场景: 调试斜杠不已识别
      当 以主机泵在 idle 提交 "/debug"
      那么 按未知斜杠提示且未换会话
      当 提交 "/debug session-tree-multiturn"
      那么 按未知斜杠提示且未注入场景
      当 提交 "/debug:session-tree-multiturn"
      那么 按未知斜杠提示
  @req:r1202
  规则: slash-session-tree-fork
    产品 TUI idle 时 须解析 /session-tree 与 /session-fork：/session-tree 须打开 MessageHistory 会话树（与双 Esc 同路径）；/session-fork 须对当前 session leaf 执行与树内 Shift+F 相同的产品 fork（user→Before、非 user→At）并 switch 到 child；busy 时二者 须拒绝并提示且 不得开树或 fork；SlashCommandSource 须列出 session-tree 与 session-fork；不得再将 /tree 或 /fork 识别为上述命令；不得发明平行 fork 语义（含 pi 式 user 消息选择器）。

    场景: 斜杠会话树打开树
      当 以主机泵在 idle 提交 "/session-tree"
      那么 会话树槽打开且经驱动取一次树
      当 提交 "/tree"
      那么 短名不被识别且树未重复打开
  @req:r1203
  规则: commands-module-growth
    产品 TUI slash/bang 解析 须继续收口于 commands 模块（可多文件），执行仍经 pending + drain_pending → protocol::Command / dispatch 或 Driver；SlashCommandSource 补全清单 须与产品命令 SSOT（及 GetCommands 内建子集）同源；不得在 UiRoot/widgets 内散落第二套 slash 语义；不得维护与 agent 短名表分叉的第三套产品名表。

    场景: 单一斜杠解析器于命令模块
      当 在产品命令模块解析 "/tree" 与 "/session-tree"
      那么 仅 session-tree 得到待发命令且废弃短名不解析
  @req:r1204
  规则: slash-session-io
    产品 TUI idle 时 须解析 /session-compact、/session-export 与 /session-import：/session-compact 无参或仅空白 须经 dispatch 调用 Compact（force；instructions=None；对齐 domain-compaction c17/c24）并以系统行提示是否压缩或明确错误；带非空参数 须调用 Compact 且 instructions 为去壳后文本（对齐 c24 Additional focus），不得再报 custom instructions not supported；/session-export 可选 path 须默认导出 HTML（对齐 pi；无参时可用 Driver/dispatch 默认 export.html），path 以 .jsonl 结尾 须走 ExportJsonl，否则 ExportHtml，成功 须回报写入路径；/session-import 缺 path 须 usage 错误，有 path 须先以 editor 槽 Yes/No 确认再 ImportJsonl 并 switch/重建 transcript，取消 不得导入；busy 时 /session-compact 与 /session-export 须仍可执行（BusySlashPolicy Allow）且 不得入 steer；/session-import busy 须拒绝且 不得入 steer；SlashCommandSource 须列出三者；旧名 /compact /export /import 不得被识别；须走 app::core::dispatch，不得发明平行词表。

    场景: 会话IO分发按后缀
      当 以主机泵在 idle 提交 "/session-compact"
      并且 提交 "/session-export dump.jsonl"
      并且 提交 "/session-export dump.html"
      那么 压缩一次且 jsonl 与 html 按后缀分派导出
  @req:r1205
  规则: slash-session-info
    产品 TUI 提交无参 /session 时 须经 dispatch 调用 GetSessionStats（及必要 GetState）并将会话信息与统计以 scrollback/系统文本块展示（对齐 pi：信息转储而非操作菜单）；带参数 须短错误或不识别为本命令；busy 时 须仍可执行（Allow）且 不得入 steer；SlashCommandSource 须列出 session；不得实现 /session 子命令操作板。

    场景: 会话信息转储作为系统文本
      当 以主机泵在 idle 提交 "/session"
      那么 以系统文本块展示会话信息与统计
  @req:r1189
  规则: slash-session-resume
    产品 TUI idle 提交无参 /session-resume 时 须打开替换 editor 槽的 Resume 面板（非居中 overlay）：数据经 Command::ListSessions 或等价 seam；面板 须提供 header（scope Current/All、Name All/Named、Sort Threaded/Recent/Fuzzy）、搜索（支持 re: 正则与引号 phrase）与会话行（name 或 first_message 预览、message_count、相对时间；Threaded 时 parent 树前缀）。预览列宽度 须按终端比例软顶（约 60% 可用宽，扣除 meta 与可选 id 列后）；超长 须 … 截断。默认 不得在会话行展示 session id；经 Ctrl+U（或键位表 app.session.toggleId）打开 id 列时 须完整显示 session id 且 不得截断。无 name/first_message 时预览 须用占位（如 —）且 不得用 id 顶替预览列。Enter 选定：idle 时 须 SwitchSession 并重建 transcript、关闭面板；agent/bang busy 时 不得 SwitchSession，须经通知条（toast notice，见 atc22）显示可见文案恰好为 `Error: ` + `agent busy — finish turn or Esc abort before switching session`（body 为文档约定等价常量；前缀由渲染层拼），不得为此追加 UiEntry::ScrollNotice；面板可保持打开；Esc 取消 不得切换；busy 时无参 /session-resume 须仍可打开面板供浏览/搜索（Allow）；旧名 /resume 不得被识别；MVP 不得要求路径直参；TUI 不得 reach infra::session。

    场景: 会话续跑打开面板经缝
      当 以主机泵注入可恢复会话列表后提交 "/session-resume"
      那么 Resume 面板槽打开且经驱动列举可恢复会话
  @req:r1190
  规则: slash-session-lifecycle
    产品 TUI idle 时 须解析 /session-new、/session-clone 与 /session-name：/session-new 无参 须经 Command::NewSession（或等价）创建空会话并切换、清空 transcript，带参 须 usage 错误；/session-clone 无参 须对当前 leaf 以 ForkPosition::At 执行 Command::Fork 并 Command::SwitchSession（对齐 pi /clone；不得套用 /session-fork 的 Before/At 规则），无 leaf 须提示且 不得 fork，带参 须 usage 错误；/session-name 无参 须显示当前会话名或 usage，有参 须经 Command::SetSessionName（换行→空格 trim）写入并回报最终名，规范化与输入不同时 须提示；busy 时 /session-new 与 /session-clone 须拒绝且 不得入 steer；/session-name busy 须仍可执行（Allow）且 不得入 steer；SlashCommandSource 须列出三者；旧名 /new /clone /name 不得被识别；不得发明平行词表或 pi 式 user 选择器。

    场景: 会话生命周期动词经缝
      当 注入会话 leaf 条目并以主机泵在 idle 提交 "/session-clone"
      并且 提交 "/session-new"
      并且 提交 "/session-name 名字"
      那么 新建一次且 clone 走 Fork(At) 且命名经 seam 写入
  @req:r1191
  规则: slash-reload
    产品 TUI idle 时 须解析无参 /reload：触发当前已注册的宿主热重载机制，并按端侧本地机制重载 keybindings、themes，以滚动提示汇总各步骤成功/失败诊断；宿主机制集合 须 由注册表决定。agent/bang busy 时 须拒绝且 不得调用重载。启动后的进行中交互（status `Reloading`、输入软闸、Esc/Ctrl+C 协作取消、失败/取消可见）须满足 ath28/atc25/ati43。不得改写已持久化 session 历史或清空已渲染 transcript 历史块；SlashCommandSource 须列出 reload；带参 须 usage 错误或不识别为成功路径。

    场景: 重载空闲触发忙碌拒绝
      当 以主机泵在 idle 提交 "/reload"
      那么 运行时重载调用恰一次
      当 以主机泵开启忙碌流并提交 "/reload"
      那么 重载被拒绝且未发第二次运行时重载
  @req:r1192
  规则: slash-trust
    产品 TUI idle 时 须解析 /trust（及文档约定子命令 self/this_dir/parent/deny）：经 Driver 缝将信任决策写入持久 trust store，并以系统块提示需 /reload 或重启后项目资源才按新信任生效；busy 时 须拒绝且 不得写盘；不得在成功路径自动调用 runtime reload；SlashCommandSource 须列出 trust；/trust 空格后 须提供 self（默认）/parent/deny 参数补全；未知子命令 须 usage 错误。

    场景: 信任持久化经Driver缝
      当 以主机泵在 idle 提交 "/trust"
      那么 信任决策经驱动持久化一次且提示需重载生效
  @req:r1193
  规则: slash-history-copy-last
    产品 TUI 须解析无参 /history-copy-last：将当前 transcript 最近一条已提交 assistant 正文（自尾向前第一条非空，不得取 thinking/tool/system 块或未提交流式半句）经 Driver 缝复制到系统剪贴板。成功 须经通知栈展示 `Copied Last Msg`（✓ 起头，见 r1930），不得再尾插 Copied last assistant message 滚动提示。无 assistant 或复制失败 须仍以滚动提示说明且 不得伪造成功、不得臂装通知栈。idle 与 busy 须均可执行（只读）；SlashCommandSource 须列出 history-copy-last；带参 须 usage 错误（ath r1250 并入）。

    场景: 历史复制最后复制助手文本
      当 以主机泵完成一轮含已提交 assistant 的对话后提交 "/history-copy-last"
      那么 剪贴板收到 assistant 正文且未复制 thinking
    场景: 历史复制最后成功栈
      当 以主机泵完成一轮含已提交 assistant 的对话后提交 "/history-copy-last"
      那么 右上角通知栈含 Copied Last Msg 且不以 Error 前缀冒充
  @req:r1194
  规则: slash-theme
    产品 TUI idle 时 须解析 /theme：无参 须打开替换 editor 槽的主题 SelectList（至少内建 dark/light）；有参 dark|light 须经 HostSession::reload_themes（或等价）应用；有参 toggle|cycle 须在 dark↔light 间切换；busy 时 须仍可打开主题列表或应用有参 dark|light|toggle|cycle（BusySlashPolicy Allow）；未知参 须提示且 不得伪造成功；SlashCommandSource 须列出 theme；空格后参数补全 须至少含 dark 与 light。

    场景: 主题槽与直接应用
      当 以主机泵在 idle 提交 "/theme"
      那么 主题列表槽打开
      当 提交 "/theme dark"
      那么 主题直接应用且槽关闭
  @req:r1195
  规则: busy-slash-policy
    产品 TUI agent/bang 忙碌且无 overlay 时，Enter 提交经 parse_slash_command 命中的 PendingSlash 须经单一穷尽 BusySlashPolicy::{Allow,Reject} 分流且 不得落入 steer/follow-up：Allow 至少含 SessionName、SetModel、HistoryCopyLast、SessionDump、Export、Exit、Compact、OpenMcp、Theme、OpenModels、OpenSessionResume；Reject 至少含 Reload、Trust、OpenTree、ForkAtLeaf、SessionNew、SessionClone、Import、Usage。OpenSessionResume Allow 仅覆盖打开/浏览；busy 下面板内 SwitchSession（及会写盘的 rename/delete 确认）须另闸拒绝并经通知条提示（见 atm10 / atc22），不得因 Allow 开面板而放行 switch。未识别且 looks_like_unknown_slash 的 `/…` 须短提示拒绝且 不得入 steer（严于 pi）。try_busy_input 不得再对个别命令散落特判。

    场景: 忙碌斜杠允许与拒绝
      当 以主机泵开启忙碌流并提交 "/model"
      那么 模型列表槽仍可打开
      当 以主机泵开启忙碌流并提交 "/reload"
      那么 重载被拒绝且未发第二次运行时重载
  @req:r1196
  规则: slash-mcp
    产品 TUI 须解析无参 /mcp（可选别名 /mcps → 同路径）：打开替换 editor 槽的 MCP SelectList（对齐 /model|/session-resume；不得居中 overlay）。列表 须支持 ↑↓ 选中（焦点行 reverse）；行至少含 server id、连接态（connecting/connected/failed 或等价）、tools armed、可选 tool 数；汇总至少含 configured / connected / armed；失败短诊断可感。Esc 关槽 不得仅因开面板 abort agent；本波 Enter 须关槽（不得假实现禁用 MCP）。开槽 须优先使用已缓存的 LoadedResourcesSnapshot（如 UiRoot/host 经 refresh 持有的）同步挂载，不得在缓存可用时仍阻塞等待完整 reconnect 式 discover；缓存缺失时 MAY await 一次 snapshot。idle、agent-busy、bang-busy 与 MCP connecting 下 须均可打开。数据 须经 Driver 只读缝（或公开等价），不得从 app/tui reach infra::mcp。SlashCommandSource 须列出 mcp；带参 须 usage 错误或不识别为成功路径；busy 时 OpenMcp 须为 BusySlashPolicy Allow 且 不得入 steer。

    场景: MCP面板打开来自已缓存快照
      当 以主机泵注入含连接态的资源快照后提交 "/mcp"
      那么 MCP 面板槽打开且列出连接态且未因开面板 abort agent
  @req:r2954
  规则: slash-plan-mode
    产品 TUI 须解析 `/plan`（toggle）、`/plan on`（plan）与 `/plan off`（execute），经共享 dispatch 提交 `SetPlanMode`；不得在端侧直接改写模式或伪造状态。成功 须以滚动提示报告当前模式（含 plan_id 时一并显示）；失败 须滚动提示错误。busy 时 须为 BusySlashPolicy Allow 且 Exec=Inline；带其他参数 须 usage 错误。模式指示渲染见 app-tui-fixed-zone 的 ModeStrip。

    场景: /plan 三态经共享 dispatch
      假如 用户在 idle 输入 /plan 或 /plan on/off
      当 解析斜杠命令
      那么 经 SetPlanMode 提交且以滚动提示报告当前模式

  @req:r1197
  规则: command-execution-class
    每条 protocol::Command 须经单一 SSOT 声明唯一执行类 Exec（Exclusive=独占会话循环直至完成，至少 bash/prompt/reload；Inline=任何交互循环内立即生效且 effect 不得 await 远程 unary/HTTP；Queued=保持排队至循环归还），声明 须与 wire REGISTRY 属性行同源并由守卫测试锁定逐变体穷举一致；客户端（泵循环/effects）须经 exec_class 推导消费，不得逐命令散落特判（延续 atm16 收口方向）；新增命令 须在 REGISTRY 行声明执行类且 不得为表达执行语义修改任何循环结构；Inline 的非阻塞承诺 须以缓存/本地直出实现（/model open 缓存先挂载为样板）。

    场景: bang行内效果运行期间bang
      假如 交互 bang 正在运行且驱动为 ScriptedDriver
      当 提交声明为 Inline 的无参 /model
      那么 模型列表 effect 在 bang 结束前挂载生效

  @req:r2984
  规则: slash-capabilities
    产品 TUI 须解析无参 `/capabilities`（别名 `/caps`）：打开替换 editor 槽的能力覆盖盘 SelectList（对齐 /mcp），汇总 须含 enabled/total 与待生效状态；行 须含能力 id、类型（tool/mcp）与开关态。Enter 须 toggle 当前项并经共享 dispatch 提交 `SetCapability`，槽 须保持打开并刷新；Esc 关槽 不得 abort agent。存在待生效覆盖时 status 区 须显示 `Next turn: capability overlay`。带参 须 usage 错误；busy 时 须为 BusySlashPolicy Allow 且 Exec=Inline。SlashCommandSource 须列出 capabilities。

    场景: 能力覆盖盘打开并列出条目
      当 以主机泵注入能力快照后提交 "/capabilities"
      那么 能力覆盖盘打开且列出能力 id 与汇总
      当 以主机泵在能力覆盖盘上切换首项
      那么 能力开关经共享分发提交且待生效提示出现

    场景: Esc关闭能力覆盖盘
      当 以主机泵注入能力快照后提交 "/capabilities"
      那么 能力覆盖盘打开且列出能力 id 与汇总
      当 以主机泵在能力覆盖盘上按 Esc
      那么 能力覆盖盘关闭且未 abort agent
  @req:r3029
  规则: plugin-slash-commands
    产品 TUI idle 时 必须 解析 `/plugin:<name> [args]` 并派发插件命令；busy 时 必须 拒绝；结果（含失败）必须 以滚动提示呈现，不得 写入模型上下文；`SlashCommandSource` 必须 合并宿主返回的 `plugin:` 前缀命令。由 harness 覆盖。
