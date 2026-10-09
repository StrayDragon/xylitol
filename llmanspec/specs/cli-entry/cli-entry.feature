# language: zh-CN
# capability: cli-entry
# purpose: CLI 统一入口：默认 TUI；surface（tui/print）与 ops（resources/serve/tokenizer）；bootstrap/dispatch；产品 slash 以 session-* 为准。
# scope: src/app/cli/, src/main.rs, tests/

功能: cli-entry

  # c2827 复核：SlashCommandInfo 类型全仓无实现（pt3 已删除 prompts 源）；本条与 r68 为移除候选，待用户裁决。
  @req:r67
  @req:r69
  规则: builtin-table
    产品对外 slash 与 GetCommands 内建表 MUST 同源自同一产品命令 SSOT（见 app-tui-commands）；名称 MUST 以 session-* /model /exit 等产品名为准（PI_DELTAS A03）；MUST NOT 再以 pi 短名表（/tree /fork /export /compact /resume /new 等）作为 GetCommands 或文档的权威清单；产品解析 MUST NOT 将上述废弃短名识别为有效命令入口。

    场景: legacy-short-names-not-parsed
      当 在产品命令模块解析 "/tree" 与 "/session-tree"
      那么 仅 session-tree 得到待发命令且废弃短名不解析

    场景: builtin-table-product-names
      假如 空扩展命令的能力聚合体
      当 调用 get_commands
      那么 含 session-tree 且不含短名 tree 作为内建主名
  @req:r70
  规则: dispatch
    产品 TUI idle slash MUST 经 commands→effects→Driver/dispatch 共享路径，MUST NOT 复制第二套命令执行语义；MUST NOT 依赖已移除的 stdio rpc。

    场景: idle-slash-parsed-into-pending-commands
      当 在产品命令模块解析 "/model" 与 "/exit"
      那么 分别得到模型与退出待发命令
  @req:r1379
  规则: app-directory
    CLI MUST 为独立应用表面，print 为其子模式；app 面 MUST NOT reach agent 内部（经缝）。
    # verified-by: src/AGENTS.md

    场景: cli-is-independent-surface
      当 xylitol --help
      那么 Commands 含 tokenizer、resources 与 serve 为顶层而非 tui 子命令

    场景: app-surface-reaches-agent-via-driver-seam
      当 检查 Driver 实现
      那么 默认 attach 且同进程驱动路径仍保留给 print 与嵌入
  @req:r1389
  规则: no-silent-fallback
    配置文件存在但 models.models 无任何显式模型别名（或等价零显式条目）时，经 bootstrap 的表面（TUI/print/--list-models/serve）MUST 硬失败并明确报错指向配置，MUST NOT 静默用 provider 环境变量注册或选中默认模型（如 gpt-4o）后继续。无配置文件且无显式模型时同样 MUST NOT 仅凭 OPENAI_API_KEY/ANTHROPIC_API_KEY 造模继续。

    场景: zero-model-config-hard-fails
      假如 存在零显式模型的配置文件
      当 调用 load_app_config 加载该文件
      那么 返回硬错误且指向配置而非回退 env 默认模型
  @req:r1392
  规则: composition-root-ports
    组合根 MUST 从 infra 构造 SessionStore/EventSink 注入装配。
    # verified-by: src/app/core/bootstrap.rs

    场景: composition-builds-ports-from-infra
      当 server 应用面启动
      那么 装配 infra 运行时并按 session 槽注入 Driver，供 unary 处理器调用
  @req:r1393
  规则: serve-subcommand
    CLI MUST 提供顶层动词 serve：无子命令时直接监听（默认 127.0.0.1:18790）；MUST 接受 --host 与 --port（--port 0 让操作系统分配并打印实际端口）；叶子至少含 stop 与 install。MUST NOT 提供 server 或 serve run 作为产品入口别名。

    场景: serve-ops-verb
      假如 CLI 已解析
      当 xylitol serve --help
      那么 可见 --host --port 与 stop、install，且无 server 或 run 别名
  @req:r1394
  规则: assembly-via-bootstrap
    cli 与 Host 监听器 MUST 经共享 bootstrap 装配，MUST NOT 内联第二套装配。
    # verified-by: src/app/core/bootstrap.rs

    场景: cli-and-server-share-one-bootstrap
      当 app::server 运行时经 app::core::composition 组装 Agent
      那么 向 Agent 传入 Arc<dyn ExportIo>
  @req:r1381
  规则: default-tui-entry
    TTY 且无表面子命令时 MUST 默认进入产品 TUI。

    场景: surface-tui-verb
      假如 TTY
      当 xylitol tui
      那么 进入产品 TUI
  @req:r1382
  规则: print-requires-prompt
    print MUST 有非空 prompt；MUST NOT 使用 Hello! 占位。

    场景: surface-print-verb
      假如 无 prompt
      当 xylitol print
      那么 错误退出且无 Hello!
  @req:r1378
  规则: session-tree-kind-api
    Command 执行器（SessionTree / TravelSessionTree）MUST 支持 SessionTreeKind 区分；至少 MessageHistory；未实现 kind MUST 明确错误。

    场景: unsupported-tree-kind-explicit-error
      当 对接好会话的驱动请求 file_browser 会话树
      那么 返回明确的不支持错误且提及 file_browser
  @req:r1383
  规则: stable-session-id-on-run
    InProcessDriver::run MUST 复用 bootstrap session_id，MUST NOT 每轮新建 Uuid 文件。
    # verified-by: llmanspec/specs/agent-session/agent-session.feature

    场景: run-reuses-bootstrap-session-id
      假如 当前 session 已出现在 list_sessions
      当 TUI 或 print 正常退出
      那么 stderr 含 resume 提示行

    场景: unpersisted-session-no-resume-hint
      假如 当前 session 未持久化
      当 TUI 或 print 正常退出
      那么 stderr 不含 resume 提示行
  @req:r1384
  规则: tokenizer-cache-subcommands
    CLI MUST 提供顶层动词 tokenizer（跨面 ops，MUST NOT 仅挂在 tui 下），叶子至少含 status、download、clean：status MUST 可报告缓存根与条目（及可选 --model 的 builtin/local/cached/missing/unmapped）；download MUST 为显式 opt-in（调用即同意；TTY 默认可二次确认，--yes 跳过），MUST 在确认摘要中告知 repo/file、HF 基址与落盘路径，MUST NOT 在估计热路径静默下载；无配置映射且无显式 owner/repo 时 MUST 失败并提示配置 tokenizer 字段或 CLI 形状，MUST NOT 猜测下载目标；clean MUST 支持 --all 或按 model/target 删除；上述子命令 MUST 早退且 MUST NOT 经 bootstrap 装配会话/MCP；实现 MUST 委托 bridge 缓存 API，MUST NOT 在 CLI 内直接发起 HTTP。

    场景: tokenizer-help-tree
      假如 CLI 已解析
      当 xylitol tokenizer --help
      那么 可见 status、download、clean 叶子

    场景: tokenizer-status-empty
      假如 缓存目录为空
      当 xylitol tokenizer status
      那么 报告缓存根且不失败伪装已下载

    场景: tokenizer-download-opt-in
      假如 目标映射到 HuggingFace 且本地无缓存
      当 xylitol tokenizer download <target> --yes
      那么 词表落入缓存路径且再次 status 可见

    场景: tokenizer-clean
      假如 缓存中已有条目
      当 xylitol tokenizer clean --all
      那么 条目被移除且 status 不再列出

    场景: tokenizer-no-bootstrap
      假如 仅执行 tokenizer 子命令
      当 xylitol tokenizer status
      那么 不经 bootstrap 装配会话或 MCP 即可完成

    场景: tokenizer-download-shows-hf-base
      假如 已设置 HF_ENDPOINT 为镜像基址且目标已映射
      当 xylitol tokenizer download <target> 进入确认摘要（或 --yes 的等价日志）
      那么 摘要含该镜像基址与落盘路径
  @req:r1385
  规则: cli-surface-vs-ops
    CLI MUST 区分表面动词与管理动词：表面至少含 tui 与 print（xylitol tui 进入 TUI；xylitol print 进入一次性 print 且须非空 prompt）；管理动词 resources、serve、tokenizer MUST 保持顶层，MUST NOT 挪入 tui 子树；TTY 裸跑默认 TUI 的语义 MUST 保持；MUST NOT 提供顶层 --tui/--print/-p/--prompt 或位置 PROMPT 作为表面切换别名（print 的 --prompt/-p 仅允许挂在 print 子命令下）。

    场景: ops-stay-toplevel
      假如 CLI 已解析
      当 xylitol --help
      那么 Commands 含 tokenizer、resources 与 serve 为顶层而非 tui 子命令
  @req:r1386
  规则: config-load-fail-closed
    load_app_config（或等价）失败（含模板渲染、YAML 解析、IO）时，bootstrap MUST 返回硬错误且经其装配的全表面 MUST 非零退出；MUST NOT 仅 Warning 后继续并用 env 默认模型进入 TUI/print。

    场景: broken-config-hard-fails
      假如 存在损坏的 YAML 配置文件
      当 调用 load_app_config 加载该文件
      那么 返回硬错误而非警告后继续
  @req:r1387
  规则: unset-model-display
    未显式选择模型时，若存在可解析的**用户显式配置默认模型**（如 models.default_model 或等价 profile 默认），产品面 MUST 将该解析默认模型展示为当前模型而非 NOT-SET；仅当既无显式选择、又无可解析的已配置默认时，产品面 MUST 展示 NOT-SET（或文档化的等价明确占位）。MUST NOT 将仅凭 provider 环境变量（如 OPENAI_API_KEY/ANTHROPIC_API_KEY 存在）即可映射的厂商默认模型（gpt-4o 等）当成已解析默认展示。

    场景: unset-model-shown-as-not-set
      当 产品面读取未选中模型的展示名
      那么 得到 NOT-SET 而非厂商默认模型名
    场景: configured-default-shown
      假如 已显式配置默认模型 fake-model 且未显式选择模型
      当 产品面读取当前模型展示名
      那么 展示为 fake-model 而非 NOT-SET
  @req:r1388
  规则: surface-owned-flags
    CLI MUST 将表面旗标挂在表面动词下：`xylitol tui`（含可选 `run`）MUST 接受 --session/--model/--list-models/--trust/--no-trust/--config/--no-color/--attach/--port；`xylitol print` MUST 接受 --session/--model/--config/--no-color/--trust/--no-trust 且 MUST NOT 接受 --list-models/--attach/--port；顶层 MUST NOT 再提供上述旗标（解析 MUST 失败）。print 的 --trust/--no-trust MUST 经 bootstrap trust_override 生效（interactive=false，MUST NOT 弹出 ChoicePrompt）。`tui --session` 与 `tui run --session` MUST 等价生效。TUI 的 --attach 指定 Host URL（默认 http://127.0.0.1:18790）；仅 --port 时 MUST 连 http://127.0.0.1:<port>；二者都给时 --attach 胜。

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
    TUI 与 print 正常退出后，若当前 session_id 出现在 Command::ListSessions（或等价已持久化判定）中，CLI MUST 向 stderr 打印恰好一行 `Resume by $ xylitol tui --session <uuid>`；未持久化或无 session_id 时 MUST NOT 打印该行。

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
    产品 TUI（含 TTY 默认进入 TUI 表面）在 attach 目标 Host 未在听时 MUST 非零退出，并提示用 xylitol serve 启动 Host。MUST NOT 静默改走同进程直握操作器。print 与库嵌入 MUST NOT 适用本条。

    场景: tui-attach-host-down
      假如 本机 Host 未在听
      当 产品 TUI 尝试 attach
      那么 非零退出并提示用 xylitol serve 启动 Host

# re-review(c2827): 复审结论——本 capability 管辖行为不变；分支内改动为 BDD 场景落地、BDD 测试基建（steps/bindings/驱动旋钮与探针）与可见性再导出（2026-09-28）
# re-review(c2835): 复审结论——本 capability 管辖行为不变；仅 OTLP 构建步骤的测试判据隔离 env，CLI 入口语义未动。（2026-09-29）

# re-review(c2837): c2837 编译隔离变更影响本 scope——agent/infra 公开化与 BDD 测试辅助面收敛（纯可见性扩张与测试基建，无行为变化）。场景映射不变量保持；已复核。（2026-10-06）

# re-review(c2837): flaky-fix 分支复核——测试时序放宽与诊断增强触及本 scope；行为不变。（2026-10-06）
