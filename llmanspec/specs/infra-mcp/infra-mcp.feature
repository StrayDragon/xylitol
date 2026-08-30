# language: zh-CN
# capability: infra-mcp
# purpose: MCP 客户端：stdio / url·sse 传输、零配置零成本、工具前缀与进程内重载。
# scope: src/infra/mcp/, src/app/

功能: infra-mcp

  @req:mcp1 @human
  场景: 无配置零装配
    - 当配置中不存在 mcp_servers 或列表为空时，System MUST NOT 创建 MCP client、MUST NOT 向默认 ToolSet 注册任何 mcp__ 前缀工具（过渡期亦可识别遗留 mcp-/mcp_/mcp:），且该路径 MUST 不引入可感知的运行时开销（zero-cost 未启用）。

  @req:mcp2 @human
  场景: 有配置则装配为 XyTool
    - 当 mcp_servers 含有效条目时，System MUST 按 transport（至少 Stdio 与 Sse/url）连接对应服务器，并将发现的工具以 XyTool 形式注册，公开名 MUST 使用 mcp__{server_id}__{name} 约定（分隔符单点可换；仅 [a-zA-Z0-9_-]，与 DeepSeek/Anthropic/OpenAI-compat tools[].name 模式对齐；禁止冒号/点号分隔；段内 MAY 保留 -/_）；execute MUST 使用保存的 server_id/tool_name，MUST NOT 反解析公开名。

  @req:mcp3 @human
  场景: 动态配置重载
    - System MUST 支持在进程存活期间根据更新后的 mcp_servers 配置重载 MCP 工具集（增删服务器/工具）；重载 MUST NOT 要求以重启整个进程作为唯一手段；重载 MUST NOT 删除内置非 mcp 工具。

  @req:mcp4 @human
  场景: 配置校验与失败可观测
    - System MUST 校验 mcp_servers 条目（stdio 需 command；url/sse 需合法 url 类字段）；无效条目 MUST 产生可读错误或诊断且 MUST NOT 静默当作成功连接；单个服务器连接失败 MUST 可观察（warn 或诊断）且 MUST NOT 单独导致整次 bootstrap 失败。

  @req:mcp5 @human
  场景: 已连接列表只读
    - System MUST 提供只读查询已成功连接的 MCP 服务器摘要（至少 id 与工具数量或等价）；无配置或全部失败时 MUST 返回空列表；该查询 MUST NOT 创建新连接。

  @req:mcp6 @human
  场景: mcp-tool-hard-barrier
    - 经 McpToolAdapter（或等价）暴露的 MCP 工具 MUST 在批调度中为 Barrier（不可进并行窗）；重载后新发现工具 MUST 同样为 Barrier；MUST NOT 提供配置名、glob 或 annotation 放行路径。由单测覆盖，MUST NOT 为静态默认单独扩 BDD step。

  @req:mcp7 @human
  场景: nonblocking-ui-parallel-startup
    - 当 mcp_servers 非空时，产品启动路径 MUST NOT 在打开 TUI 应用面（或 print 进入可观测启动）之前阻塞等待全部 MCP 连接完成；CLI `--session` resume MUST 能在 MCP 未完成时投影历史。多服务器连接 MUST 并行尝试（单失败 MUST 仍可观察且 MUST NOT 拖死其余，对齐 mcp4）。连接过程 MUST 经 Driver/composition 只读缝暴露可轮询的进度或快照（至少：配置数、已连接摘要、进行中/完成态或等价；每 server 或汇总的 tools armed 态，供 `/mcp` 面板与短 cue）。TUI 在 MCP 未结算前 MUST NOT 因 connecting 拒绝用户键入、提交普通 agent prompt、bang 或 slash（含面内 session-resume、`/reload` 与 `/mcp`）（agent busy 既有闸除外）；MUST 允许滚历史。Host 为 session 物化写者或处理 SetModel 等 unary 时 MUST NOT 在该 unary 返回前等待全部 MCP 连接完成（门闸仍见 mcp8，跨 tick）。结算成功 MUST 经单一入口更新内部 MCP registry/armed 快照（保留内置、同名不重复）；**MUST NOT** 在工具表已定稿（FROZEN）后静默把新 settle 结果热扩进 provider 可见 `tools[]`（定稿/重定稿语义见 mcp8）。替换 manager 前 MUST shutdown 旧 MCP client/子进程，MUST NOT 静默堆叠无法管理的连接进程。MUST NOT 往 session 历史插入宣告 MCP ready 的假 system/user 行。无配置路径 MUST 仍满足 mcp1 zero-cost。

  @req:mcp9 @human
  场景: loaded-resources-live-mcp
    - 当 mcp_servers 非空时，只读 loaded_resources 快照 MUST 与进程内正在连接或已连接的 MCP 状态同源；MUST NOT 用一份从未发起连接的资源壳冒充完成态（仅 configured、connected 恒空且无 connecting/诊断）。无配置时 MUST 仍满足 mcp1。

  @req:mcp10 @human
  场景: call-phase-timeout
    - MCP 连接建立后的每笔请求（tools/list、tools/call 等）MUST 有单次调用期超时；到期以可分类超时错误呈现给调用方，MUST NOT 无限等待。

  @req:mcp8 @human
  场景: first-turn-tool-freeze
    - 轨 A（开箱定稿，非 c1960 search）：无 mcp_servers 时 MUST 立即以仅核心工具定稿。有配置时，会话首次 agent generate（及尚未 FROZEN 时）MUST 等待 MCP settle 或 code-first 门闸超时后再定稿；单 server 连接 MUST 有 code-first 墙钟超时（挂死不得让 bootstrap 永 Running）。门闸超时 MUST 以当时已武装子集定稿、收口 connecting UI、并暴露用户可见诊断（可 `/reload`），MUST NOT 自动重试连接。TUI MUST NOT 在 host tick 上阻塞长等门闸（可跨 tick Assembling）。定稿后 provider 可见工具表 MUST 冻结，直至 idle `/reload` 或 resume/切会话触发的重定稿；重定稿 MUST 按 tool name upsert（有则替换、无则追加），MUST NOT 留下同名多行。本波 resume/切会话 MUST 清冻再门闸（指纹持久化后的一致续冻另波）；重定稿 MAY 用 mcp pending / 超时 notice 作可见 cue。由单测覆盖门闸/冻结/upsert/超时收口；产品 cue 可由 host 场景覆盖。
