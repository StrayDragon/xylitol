# language: zh-CN
# purpose: MCP 客户端：stdio / url·sse 传输、零配置零成本、工具前缀与进程内重载。准备门为会话瀑布事件 session/prepare：切会话开火不 wait，generate 才 wait+超时（默认 15s）后放行；迟到工具在下一 turn 边界重新冻结。

功能: infra-mcp

  @req:r1445
  规则: 无配置零装配
    当配置中不存在 mcp_servers 或列表为空时，System 不得创建 MCP client、不得向默认 ToolSet 注册任何 mcp__ 前缀工具（过渡期亦可识别遗留 mcp-/mcp_/mcp:），且该路径 须不引入可感知的运行时开销（zero-cost 未启用）。

    场景: 无MCP配置无工具
      假如 产品以 Print 面启动
      当 查询装配后的 ToolSet
      那么 装配不含 mcp 前缀工具
  @req:r1447
  规则: 有配置则装配为 XyTool
    当 mcp_servers 含有效条目时，System 须按 transport（至少 Stdio 与 Sse/url）连接对应服务器，并将发现的工具以 XyTool 形式注册，公开名 须使用 mcp__{server_id}__{name} 约定（分隔符单点可换；仅 [a-zA-Z0-9_-]，与 DeepSeek/Anthropic/OpenAI-compat tools[].name 模式对齐；禁止冒号/点号分隔；段内 MAY 保留 -/_）；execute 须使用保存的 server_id/tool_name，不得反解析公开名。

    场景: 夹具服务端装配XyTool
      当 以 fixture MCP 配置装配 ping 工具
      那么 装配出 mcp 前缀的 XyTool 且连接在册
  @req:r1448
  规则: 动态配置重载
    System 须支持在进程存活期间根据更新后的 mcp_servers 配置重载 MCP 工具集（增删服务器/工具）；重载 不得要求以重启整个进程作为唯一手段；重载 不得删除内置非 mcp 工具。

    场景: 工具集跟随配置
      当 以两组不同工具集的 fixture 配置先后装配
      那么 工具集随配置变化
  @req:r1449
  规则: 配置校验与失败可观测
    System 须校验 mcp_servers 条目（stdio 需 command；url/sse 需合法 url 类字段）；无效条目 须产生可读错误或诊断且 不得静默当作成功连接；单个服务器连接失败 须可观察（warn 或诊断）且 不得单独导致整次 bootstrap 失败。

    场景: 无效入口已诊断不致命
      当 以无效 MCP 条目装配
      那么 诊断在册且不产出该条目工具
  @req:r1450
  规则: 已连接列表只读
    System 须提供只读查询已成功连接的 MCP 服务器摘要（至少 id 与工具数量或等价）；无配置或全部失败时 须返回空列表；该查询 不得创建新连接。

    场景: 已连接列表只读
      当 以 fixture MCP 配置装配 ping 工具
      那么 已连接列表只读返回在册服务器摘要
  @req:r1451
  规则: mcp-tool-hard-barrier
    经 McpToolAdapter（或等价）暴露的 MCP 工具 须在批调度中为 Barrier（不可进并行窗）；重载后新发现工具 须同样为 Barrier；不得提供配置名、glob 或 annotation 放行路径。由单测覆盖，不得为静态默认单独扩 BDD step。

    场景: MCP工具硬屏障调度
      当 读取 MCP 工具批调度标记
      那么 MCP 工具在批调度中为 Barrier 且不可进并行窗
  @req:r1452
  规则: nonblocking-ui-parallel-startup
    当 mcp_servers 非空时，产品启动路径 不得在打开 TUI 应用面（或 print 进入可观测启动）之前阻塞等待全部 MCP 连接完成；CLI `--session` resume 须能在 MCP 未完成时投影历史。多服务器连接 须并行尝试（单失败 须仍可观察且 不得拖死其余，对齐 mcp4）。连接过程 须经只读缝暴露可轮询的进度或快照（至少：配置数、已连接摘要、进行中/完成态或等价；每 server 或汇总的 tools armed 态，供 `/mcp` 面板与短 cue）。TUI 在 MCP 未结算前 不得因 connecting 拒绝用户键入、提交普通 agent prompt、bang 或 slash（含面内 session-resume、`/reload` 与 `/mcp`）（agent busy 既有闸除外）；须允许滚历史。Host 为 session 物化写者、resume/切会话或处理 SetModel 等 unary 时 必须开火会话准备事件且 不得在该 unary 返回前等待全部 MCP 连接完成（门闸仍见 mcp8，跨 tick）。结算成功 须经单一入口更新内部 MCP registry/armed 快照（保留内置、同名不重复）；**不得** 在工具表已定稿（FROZEN）后静默把新 settle 结果热扩进 provider 可见 `tools[]`（定稿/重定稿语义见 mcp8）。替换 manager 前 须 shutdown 旧 MCP client/子进程，不得静默堆叠无法管理的连接进程。不得往 session 历史插入宣告 MCP ready 的假聊天行（准备增量 Env 行见 agent-prompt，不是此类宣告）。无配置路径 须仍满足 mcp1 zero-cost。

    场景: 非阻塞界面并行启动
      当 读取产品启动装配顺序
      那么 MCP 连接不阻塞应用面打开
  @req:r1454
  规则: loaded-resources-live-mcp
    当 mcp_servers 非空时，只读 loaded_resources 快照 须与进程内正在连接或已连接的 MCP 状态同源；不得用一份从未发起连接的资源壳冒充完成态（仅 configured、connected 恒空且无 connecting/诊断）。无配置时 须仍满足 mcp1。

    场景: 已加载资源在线MCP来源
      当 读取已加载资源快照装配
      那么 快照与在册 MCP 状态同源
  @req:r1446
  规则: call-phase-timeout
    MCP 连接建立后的每笔请求（tools/list、tools/call 等）须有单次调用期超时；到期以可分类超时错误呈现给调用方，不得无限等待。

    场景: 调用阶段超时已分类
      当 读取 MCP 单次调用超时配置
      那么 每笔请求有调用期超时且可分类
  @req:r1453
  规则: first-turn-tool-freeze
    轨 A（开箱定稿，非 c1960 search）：无 mcp_servers 时 须立即以仅核心工具定稿。有配置时，会话首次 agent generate（及尚未 FROZEN 时）必须等待会话准备事件 settle 或 code-first 门闸超时后再定稿；单 server 连接 须有 code-first 墙钟超时（挂死不得让准备永 Running）。门闸超时 须以当时已武装子集定稿、收口 connecting UI、并暴露用户可见诊断（可 `/reload`），不得自动重试连接。TUI 不得在 host tick 上阻塞长等门闸（可跨 tick Assembling）。定稿后 provider 可见工具表 须冻结，直至 idle `/reload` 或 resume/切会话触发的重定稿。idle `/reload` 与 resume/切会话 必须清冻、开火准备事件且 unary 不得 wait；下一次 generate 才 wait 并按 tool name upsert 重定稿（有则替换、无则追加），不得留下同名多行。正确性优先于供应商 KV cache：准备结果有变时须在下一用户提问前追加 Env 行（见 agent-prompt），不得改写既有 turn；无变不得追加。重定稿 MAY 用 mcp pending / 超时 notice 作可见 cue。由单测覆盖门闸/冻结/upsert/超时收口；产品 cue 可由 host 场景覆盖。

    场景: 首次回合工具冻结门禁
      当 读取首回合工具定稿门禁
      那么 无配置立即定稿且有配置时首回合后门闸定稿

  @req:r2934
  规则: mcp-result-truncate
    MCP 工具结果 须有单次输出上限：超过上限的单段文本结果 须在进入会话历史之前被截断，并在截断处标注保留与丢弃字节数及上限，不得静默丢内容。上限 须与既有内置工具截断同一口径（50 KiB），不得复用观测 IO 常量。含图片等多分片的工具结果 须整体保留，不得为截断而拍平结构化分片。错误结果 与非 MCP 工具 不得被本上限截断。截断能力 须由 MCP 侧装配（有 servers 才注册），不得硬编码进工具适配器，也须可卸载。

    场景: 超限输出被截断
      假如 MCP 工具返回超过上限的单段文本
      当 该结果经过截断中间件
      那么 结果变短且含截断提示

    场景: 多分片保真
      假如 MCP 工具返回文本加图片分片
      当 该结果经过截断中间件
      那么 分片数量与图片分片保持不变

  @req:r3227
  规则: mcp-scope-pool
    MCP 连接 须按 connection key =（config fingerprint, scope, credential, workspace, session）池化；McpScope 含 Shared / Workspace / Session（缺省 Workspace）；同 key 共用一个连接；懒连接 + 引用计数，最后一个 claim 释放才关闭。由单测覆盖，不扩 BDD step。

  @req:r3228
  规则: mcp-lifecycle-no-reset
    Switch / Resume 不得 reset MCP pool；只有 Reload 清空并重连；单 server 连接失败按 server 重连且不拖死其余。由单测覆盖，不扩 BDD step。

  @req:r3229
  规则: mcp-last-known-snapshot
    pool 忙或正在重连时 view 须返回上一份已知快照；不得把未知当 configured=0。由单测覆盖，不扩 BDD step。

  @req:r3231
  规则: mcp-tool-catalog
    MCP tool catalog 须按 connection key 建立；catalog 只含 tools；同名 server 不同 config 独立；list tools 失败保留上一份并诊断。由单测覆盖，不扩 BDD step。

  @req:r3232
  规则: mcp-profile-filter
    MCP 工具须进入统一 tool selection 管线；profile / config tools 的 `+name` / `-name` 可增删 `mcp__server__tool`；不得另开第二套过滤语义。由单测覆盖，不扩 BDD step。

  @req:r3233
  规则: mcp-tool-list-changed
    server 发 `notifications/tools/list_changed` 时 须标记 catalog dirty，下一轮 prepare / ensure 重新 list 并重建 adapter；不得在当前已冻结 provider 工具表上热扩。由单测覆盖，不扩 BDD step。

  @req:r3234
  规则: mcp-resume-reconcile
    resume 须按当前 profile 与 session workspace set 重建 MCP claims 与 tool set；未使用 server 释放。由单测覆盖，不扩 BDD step。

  @req:r3294
  规则: mcp-host-resource-lease
    MCP 连接 须由 host 作为资源租约管理：session attach 获取 claim，clean detach / session eviction 释放 claim；同 key 复用；最后一个 claim 释放后，连接保留 keep_alive_ms + idle_ttl_ms 后关闭；延迟期内新 claim 须复用未关闭连接。Shared 跨 session，Workspace 同 workspace，Session 单 session 且结束即关（忽略 keep_alive_ms / idle_ttl_ms）；有状态 server 须用 Session。LoadedResourcesSnapshot 只含当前 session claim，不得聚合 idle 或其他 session 的连接。由单测覆盖，不扩 BDD step。

  @req:r3235
  规则: mcp-canonical-transport
    MCP transport 只支持 `stdio` 与 `http`（Streamable HTTP）；`transport: sse` 须破坏性拒绝并给迁移提示；legacy HTTP+SSE 不支持；不得保留兼容别名。由单测覆盖，不扩 BDD step。

  @req:r3236
  规则: mcp-per-server-timeout
    每 server 须有 `connect_timeout_ms`（缺省 8000）与 `call_timeout_ms`（缺省 120000）；connect 超时按 server 独立；call 超时按 server 覆盖全局；非法值配置校验失败；超时返回可分类错误。由单测覆盖，不扩 BDD step。

  @req:r3237
  规则: mcp-legacy-sse-diagnostic
    legacy HTTP+SSE endpoint 或协议须产生可读诊断（不自动回退），供 `/mcp` 面板显示。由单测覆盖，不扩 BDD step。

  @req:r3239
  规则: mcp-custom-transport-seam
    transport 须接受 stdio / http / custom；custom 须提供 custom_transport 名称且名称已注册；已注册但无连接器时连接须返回稳定错误；不得静默回退；不实现 WebSocket；不假设连接器 API 形状。由单测覆盖，不扩 BDD step。

  @req:r3240
  规则: mcp-prompts-resources
    MCP prompts 须以 `mcp__server__prompt__name` 暴露为模型工具，resources 须以 `mcp__server__resource__read` 暴露（uri 参数）；prompts/get 与 resources/read 须受 per-server call timeout；list 失败保留上一份 catalog 并诊断；profile tools 选择同样适用。由单测覆盖，不扩 BDD step。

  @req:r3275
  规则: mcp-prompt-ui
    TUI `/mcp prompt` 须列出当前 scope 的 prompt；有参数时 须用 ChoicePrompt 自定义输入逐项收集，必填为空 须提示并重开；`prompts/get` 结果 须插入 editor，不得自动发送；get 失败 须可见诊断并保留选择器。由单测 / harness 覆盖，不扩 BDD step。

  @req:r3276
  规则: mcp-resource-tray
    TUI `/mcp resource` 与 `@mcp:` 补全选中 须加入 editor 上方附件 tray，不得插入可解析文本 token；tray 须支持点击 `×`、空 editor Backspace、快捷键移除，并按 `(server, uri)` 去重。未加引号手写 `@mcp:` 须按普通文本；完整 `@mcp:"<server>/<uri>"` 须解析，跳过 fenced/inline code，未知资源 须阻止发送。由单测 / harness 覆盖，不扩 BDD step。

  @req:r3277
  规则: mcp-context-seam
    发送附件 须经 host `resources/read` 读取；任一失败 须阻止发送并给可操作错误。成功 须经 `XyDriver::run_with_context` 在用户消息前写 `CustomMessage`（custom_type `mcpResource`），transcript 须可见 context 来源（滚动提示或块）；`project_for_llm` 按 user text 投影。由单测 / harness 覆盖，不扩 BDD step。

  @req:r3278
  规则: mcp-resource-cache-subscription
    资源缓存 key 须为 `(server, uri)`，LRU 64、单条 256 KiB、TTL 5 分钟；`resources/updated` 须失效对应 URI，`resources/list_changed` 须清 catalog/缓存。连接后 须 `Peer::listen` 订阅 `resources_list_changed` 与已知 URI；不支持 须 warn 降级手动刷新，不得阻塞连接。由单测覆盖，不扩 BDD step。

  @req:r3279
  规则: mcp-elicitation-ui
    TUI elicitation 须用表单槽位渲染 primitive 字段（string / number / integer / boolean / enum），支持 title / description / default / required / min / max / format；秘密字段或未知形状 须整表 decline，不得渲染；accept 须校验类型与必填；等待响应 5 分钟超时 须 decline；审计只记 server / 字段名 / action / reason / timestamp，不得记值或 message。由单测 / harness 覆盖，不扩 BDD step。

  @req:r3241
  规则: mcp-elicitation-secrets
    MCP elicitation 表单不得收集秘密；requestedSchema 字段命中 password / secret / token / api_key / credential / private_key 须拒绝；默认不弹 UI，返回 decline；不得记录 message 或字段值。由单测覆盖，不扩 BDD step。

  @req:r3244
  规则: mcp-credential-key
    credential ID 须进入 connection key；同 server 不同 credential 须独立连接与 catalog；credential 值不得进入日志或工具子进程 env。由单测覆盖，不扩 BDD step。

  @req:r3245
  规则: mcp-diagnostics
    /mcp 诊断须显示每 server 的 transport / scope / credential 是否存在 / 连接状态 / tools / prompts / resources / timeout 配置 / last error；不得显示 credential 值；不得把未知当 configured=0。由单测覆盖，不扩 BDD step。

  @req:r3268
  规则: mcp-oauth-authorization
    远程 MCP 须支持 OAuth 2.1 + PKCE（S256），且只支持 public client（不得使用 client_secret）；须发现 Protected Resource Metadata 与 AS metadata，优先使用 401 challenge 的 resource_metadata；须校验 state、RFC 8707 resource 与 RFC 9207 iss；token 刷新 不得 reset 连接；refresh 被拒 须提示重新授权；scope 不足 须发起 step-up；revoke 须清 credential store；授权未完成 不得阻塞 bootstrap / 首回合 / UI，须以可恢复 auth-required 状态暴露，完成后按既有准备/重定稿语义恢复，不得静默热加工具；token 不得进入 config / secret.env / 日志 / 错误 / 导出 / 工具子进程 env；`/mcp` 只显示 credential 是否存在。由单测覆盖，不扩 BDD step。

  @req:r3269
  规则: mcp-oauth-keychain-fallback
    token 存储 须 OS keychain 优先、0600 文件 fallback；keychain 不可用 不得阻塞连接；`oauth.keychain: off` 须强制文件；credential ID 缺省须生成稳定默认账号 ID；credential ID 须进 connection key，同 server 多账号隔离。由单测覆盖，不扩 BDD step。

  @req:r3270
  规则: mcp-oauth-host-callback
    Host 须监听 localhost 一次性 callback（默认 127.0.0.1 临时端口；pre-registered client 可固定 redirect URI，固定端口不得为 0）；callback 只接受 GET；取消须终止 listener；redirect URI 须与 client 注册一致；多个并发 flow 须按 flow_id / state 隔离；TUI 须展示 URL 与同意/取消；code exchange 成功后 须保存 credential 并发完成事件，失败 须发失败事件；code/token 不得进事件或日志；TUI 与 Host 不同机器时 不得承诺 localhost callback 可达，须明确不支持远程 TUI OAuth。由单测覆盖，不扩 BDD step。

  @req:r3271
  规则: mcp-capability-probe
    连接 initialize 成功后 须异步探测 `server/discover`；不得阻塞 bootstrap / 首回合 / UI；按 connection key 缓存；TTL 取 `ttl_ms` 夹取 60s–1h；失败 须诊断 unsupported / timeout / transport / auth / invalid_response 且保留连接；legacy 不支持时沿用 initialize + tools/list。由单测覆盖，不扩 BDD step。

  @req:r3272
  规则: mcp-reconnect-count
    pool 须按 connection key 统计重建/重连次数；`/mcp` 须展示 count 与 last error；`/reload` 须重置；不得展示 token/credential 值。由单测覆盖，不扩 BDD step。

  @req:r3274
  规则: mcp-provided-name-safe
    server 提供的 tool / prompt 名导致公开名含 `[A-Za-z0-9_-]` 之外字符 须跳过并诊断；不得 debug_assert 后继续；不得静默 sanitize；诊断可 `/mcp` 查看。由单测覆盖，不扩 BDD step。

  @req:r3273
  规则: mcp-server-name-safe
    `mcp_servers[].name` 须只含 `[A-Za-z0-9_-]`；含 `/`、空白、引号、冒号、点号 须配置校验失败并给可操作错误；不得只在 adapter debug_assert。由单测覆盖，不扩 BDD step。

  @req:r3267
  规则: mcp-pending-settled
    MCP pending cue 须仅在准备未完成时显示；bootstrap complete 且 tools frozen 后 不得因旧 connecting_label 显示。freeze / arm / gate timeout / reload / resume / 切会话后 须刷新同一 loaded-resources 快照；gate timeout 须收口为 complete / frozen 并保留诊断；connect_timeout_ms 须覆盖 stdio initialize / handshake，超时收口 Failed / timed-out。

    场景: settled snapshot 清 cue
      假如 MCP 已 connected 且 armed 但 connecting_label 残留
      当 刷新 loaded-resources 快照
      那么 cue 清除

    场景: gate timeout 收口
      假如 gate timeout 后 tools frozen
      当 刷新 loaded-resources 快照
      那么 cue 清除且诊断保留

    场景: 挂死 server 超时收口
      假如 挂死 server 超过 connect_timeout_ms
      当 刷新 loaded-resources 快照
      那么 header 与面板一致且不再 connecting
