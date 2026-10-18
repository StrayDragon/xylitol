# language: zh-CN
# purpose: CLI 统一入口：默认 TUI；surface（tui/print）与 ops（serve）；bootstrap/dispatch；产品 slash 以 session-* 为准。

功能: cli-entry

  # c2827 复核：SlashCommandInfo 类型全仓无实现（pt3 已删除 prompts 源）；本条与 r68 为移除候选，待用户裁决。
  @req:r67
  @req:r69
  规则: builtin-table
    产品对外 slash 与 GetCommands 内建表 须同源自同一产品命令 SSOT（见 app-tui-commands）；名称 须以 session-* /model /exit 等产品名为准（PI_DELTAS A03）；不得再以 pi 短名表（/tree /fork /export /compact /resume /new 等）作为 GetCommands 或文档的权威清单；产品解析 不得将上述废弃短名识别为有效命令入口。

    场景: legacy-short-names-not-parsed
      当 在产品命令模块解析 "/tree" 与 "/session-tree"
      那么 仅 session-tree 得到待发命令且废弃短名不解析

    场景: builtin-table-product-names
      假如 空扩展命令的能力聚合体
      当 调用 get_commands
      那么 含 session-tree 且不含短名 tree 作为内建主名
  @req:r70
  规则: dispatch
    产品 TUI idle slash 须经 commands→effects→Driver/dispatch 共享路径，不得复制第二套命令执行语义；不得依赖已移除的 stdio rpc。

    场景: idle-slash-parsed-into-pending-commands
      当 在产品命令模块解析 "/model" 与 "/exit"
      那么 分别得到模型与退出待发命令
  @req:r1379
  规则: app-directory
    CLI 须为独立应用表面，print 为其子模式；app 面 不得 reach agent 内部（经缝）。

    场景: cli-is-independent-surface
      当 xylitol --help
      那么 Commands 含 serve 为顶层且不含 tokenizer 与 resources

    场景: app-surface-reaches-agent-via-driver-seam
      当 检查 Driver 实现
      那么 默认 attach 且同进程驱动路径仍保留给 print 与嵌入
  @req:r1389
  规则: no-silent-fallback
    配置文件存在但 models.models 无任何显式模型别名（或等价零显式条目）时，经 bootstrap 的表面（TUI/print/--list-models/serve）须硬失败并明确报错指向配置，不得静默用 provider 环境变量注册或选中默认模型（如 gpt-4o）后继续。无配置文件且无显式模型时同样 不得仅凭 OPENAI_API_KEY/ANTHROPIC_API_KEY 造模继续。

    场景: zero-model-config-hard-fails
      假如 存在零显式模型的配置文件
      当 调用 load_app_config 加载该文件
      那么 返回硬错误且指向配置而非回退 env 默认模型
  @req:r1392
  规则: composition-root-ports
    组合根 须从 infra 构造 SessionStore/EventSink 注入装配。

    场景: composition-builds-ports-from-infra
      当 server 应用面启动
      那么 装配 infra 运行时并按 session 槽注入 Driver，供 unary 处理器调用
  @req:r1393
  规则: serve-subcommand
    CLI 须提供顶层动词 serve：无子命令时直接监听（默认 127.0.0.1:18790）；须接受 --host 与 --port（--port 0 让操作系统分配并打印实际端口）；叶子含 stop。不得提供 install、server 或 serve run 作为产品入口。

    场景: serve-ops-verb
      假如 CLI 已解析
      当 xylitol serve --help
      那么 可见 --host --port 与 stop，且无 install、server 或 run 别名
  @req:r1394
  规则: assembly-via-bootstrap
    cli 与 Host 监听器 须经共享 bootstrap 装配，不得内联第二套装配。

    场景: cli-and-server-share-one-bootstrap
      当 app::server 运行时经 app::core::composition 组装 Agent
      那么 向 Agent 传入 Arc<dyn ExportIo>
  @req:r1381
  规则: default-tui-entry
    TTY 且无表面子命令时 须默认进入产品 TUI。

    场景: surface-tui-verb
      假如 TTY
      当 xylitol tui
      那么 进入产品 TUI
  @req:r1382
  规则: print-requires-prompt
    print 须有非空 prompt；不得使用 Hello! 占位。

    场景: surface-print-verb
      假如 无 prompt
      当 xylitol print
      那么 错误退出且无 Hello!
  @req:r1378
  规则: session-tree-kind-api
    Command 执行器（SessionTree / TravelSessionTree）须支持 SessionTreeKind 区分；至少 MessageHistory；未实现 kind 须明确错误。

    场景: unsupported-tree-kind-explicit-error
      当 对接好会话的驱动请求 file_browser 会话树
      那么 返回明确的不支持错误且提及 file_browser
  @req:r1383
  规则: stable-session-id-on-run
    InProcessDriver::run 须复用 bootstrap session_id，不得每轮新建 Uuid 文件。

    场景: run-reuses-bootstrap-session-id
      假如 当前 session 已出现在 list_sessions
      当 TUI 或 print 正常退出
      那么 stderr 含 resume 提示行

    场景: unpersisted-session-no-resume-hint
      假如 当前 session 未持久化
      当 TUI 或 print 正常退出
      那么 stderr 不含 resume 提示行
  @req:r1385
  规则: cli-surface-vs-ops
    CLI 须区分表面动词与管理动词：表面至少含 tui 与 print（xylitol tui 进入产品 TUI；xylitol print 进入一次性 print 且须非空 prompt）；管理动词 serve 须保持顶层，不得提供顶层 resources、tokenizer、preview 或 designing 动词，不得挪入 tui 子树；TTY 裸跑默认产品 TUI 的语义 须保持，裸跑 不得进入 designing；不得提供顶层 --tui/--print/-p/--prompt 或位置 PROMPT 作为表面切换别名（print 的 --prompt/-p 仅允许挂在 print 子命令下）。debug 构建的 tui designing 叶见 app-tui-preview，不是表面动词。

    场景: ops-stay-toplevel
      假如 CLI 已解析
      当 xylitol --help
      那么 Commands 含 serve 为顶层且不含 tokenizer、resources、preview 与 designing
  @req:r1386
  规则: config-load-fail-closed
    load_app_config（或等价）失败（含模板渲染、YAML 解析、IO）时，bootstrap 须返回硬错误且经其装配的全表面 须非零退出；不得仅 Warning 后继续并用 env 默认模型进入 TUI/print。

    场景: broken-config-hard-fails
      假如 存在损坏的 YAML 配置文件
      当 调用 load_app_config 加载该文件
      那么 返回硬错误而非警告后继续
  @req:r1387
  规则: unset-model-display
    未显式选择模型时，若存在可解析的**用户显式配置默认模型**（如 models.default_model 或等价 profile 默认），产品面 须将该解析默认模型展示为当前模型而非 NOT-SET；仅当既无显式选择、又无可解析的已配置默认时，产品面 须展示 NOT-SET（或文档化的等价明确占位）。不得将仅凭 provider 环境变量（如 OPENAI_API_KEY/ANTHROPIC_API_KEY 存在）即可映射的厂商默认模型（gpt-4o 等）当成已解析默认展示。

    场景: unset-model-shown-as-not-set
      当 产品面读取未选中模型的展示名
      那么 得到 NOT-SET 而非厂商默认模型名
    场景: configured-default-shown
      假如 已显式配置默认模型 fake-model 且未显式选择模型
      当 产品面读取当前模型展示名
      那么 展示为 fake-model 而非 NOT-SET
  @req:r1388
  规则: surface-owned-flags
    CLI 须将表面旗标挂在表面动词下：`xylitol tui`（含可选 `run`）须接受 --session/--model/--list-models/--trust/--no-trust/--config/--no-color/--attach/--port；`xylitol print` 须接受 --session/--model/--config/--no-color/--trust/--no-trust 且 不得接受 --list-models/--attach/--port；顶层 不得再提供上述旗标（解析 须失败）。print 的 --trust/--no-trust 须经 bootstrap trust_override 生效（interactive=false，不得弹出 ChoicePrompt）。`tui --session` 与 `tui run --session` 须等价生效。TUI 的 --attach 指定 Host URL（默认 http://127.0.0.1:18790）；仅 --port 时 须连 http://127.0.0.1:<port>；二者都给时 --attach 胜。

    场景: surface-flags-on-tui
      假如 表面旗标上下文就绪
      当 xylitol tui --session sid --model m --trust
      那么 解析成功且表面旗标生效

    场景: surface-flags-on-tui-run
      假如 表面旗标上下文就绪
      当 xylitol tui run --session sid
      那么 解析成功且 --session 生效

    场景: surface-flags-tui-attach
      假如 表面旗标上下文就绪
      当 xylitol tui --attach http://127.0.0.1:9 --port 11
      那么 解析成功且 --attach 优先于 --port

    场景: surface-flags-on-print
      假如 表面旗标上下文就绪
      当 xylitol print --session sid --no-color hi
      那么 解析成功

    场景: surface-flags-on-print-trust
      假如 表面旗标上下文就绪
      当 xylitol print --session sid --trust --model m hi
      那么 解析成功且表面旗标生效

    场景: surface-flags-on-print-no-trust
      假如 表面旗标上下文就绪
      当 xylitol print --session sid --no-trust hi
      那么 解析成功且表面旗标生效

    场景: toplevel-surface-flags-rejected
      假如 表面旗标上下文就绪
      当 xylitol --session sid
      那么 解析失败
  @req:r1390
  规则: resume-hint-on-exit
    TUI 与 print 正常退出后，若当前 session_id 出现在 Command::ListSessions（或等价已持久化判定）中，CLI 须向 stderr 打印恰好一行 `Resume by $ xylitol tui --session <uuid>`；未持久化或无 session_id 时 不得打印该行。

    场景: resume-hint-when-persisted
      假如 当前 session 已出现在 list_sessions
      当 TUI 或 print 正常退出
      那么 stderr 含 resume 提示行

    场景: resume-hint-absent-when-unpersisted
      假如 当前 session 未持久化
      当 TUI 或 print 正常退出
      那么 stderr 不含 resume 提示行
  @req:r1391
  规则: tui-attach-fail-closed
    产品 TUI（含 TTY 默认进入 TUI 表面）在 attach 目标 Host 未在听时 须非零退出，并提示用 xylitol serve 启动 Host。不得静默改走同进程直握操作器。print、库嵌入与 debug 构建的 tui designing 组合 不得适用本条。

    场景: tui-attach-host-down
      假如 本机 Host 未在听
      当 产品 TUI 尝试 attach
      那么 非零退出并提示用 xylitol serve 启动 Host
