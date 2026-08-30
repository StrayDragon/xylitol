# language: zh-CN
# capability: cli-entry
# purpose: CLI 统一入口：默认 TUI；surface（tui/print）与 ops（resources/serve/tokenizer）；bootstrap/dispatch；产品 slash 以 session-* 为准。
# scope: src/app/cli/, src/main.rs, tests/

功能: cli-entry

  @req:r67 @human
  场景: info-fields
    - SlashCommandInfo MUST 携带 name、description、source 与 source_info 字段。

  @req:r68 @human
  场景: source-enum
    - SlashCommandSource MUST 恰为 prompt 或 skill 之一（extension 若未实现 MUST NOT 作为产品必选项）。

  @req:r69 @human
  场景: builtin-table
    - 产品对外 slash 与 GetCommands 内建表 MUST 同源自同一产品命令 SSOT（见 app-tui-commands）；名称 MUST 以 session-* /model /exit 等产品名为准（PI_DELTAS A03）；MUST NOT 再以 pi 短名表（/tree /fork /export /compact /resume /new 等）作为 GetCommands 或文档的权威清单；产品解析 MUST NOT 将上述废弃短名识别为有效命令入口。

  @req:r70 @human
  场景: dispatch
    - 产品 TUI idle slash MUST 经 commands→effects→Driver/dispatch；MUST NOT 依赖已移除的 stdio rpc。

  @req:ce1 @human
  场景: app-directory
    - CLI MUST 为独立应用表面，print 为其子模式；app 面 MUST NOT reach agent 内部（经缝）。

  @req:ce2 @human
  场景: no-silent-fallback
    - 配置文件存在但 models.models 无任何显式模型别名（或等价零显式条目）时，经 bootstrap 的表面（TUI/print/--list-models/serve）MUST 硬失败并明确报错指向配置，MUST NOT 静默用 provider 环境变量注册或选中默认模型（如 gpt-4o）后继续。无配置文件且无显式模型时同样 MUST NOT 仅凭 OPENAI_API_KEY/ANTHROPIC_API_KEY 造模继续。

  @req:ce7 @human
  场景: composition-root-ports
    - 组合根 MUST 从 infra 构造 SessionStore/EventSink 注入装配。

  @req:ce8 @human
  场景: serve-subcommand
    - CLI MUST 提供顶层动词 serve：无子命令时直接监听（默认 127.0.0.1:18790）；MUST 接受 --host 与 --port（--port 0 让操作系统分配并打印实际端口）；叶子至少含 stop 与 install。MUST NOT 提供 server 或 serve run 作为产品入口别名。

  @req:ce9 @human
  场景: assembly-via-bootstrap
    - cli 与 Host 监听器 MUST 经共享 bootstrap 装配，MUST NOT 内联第二套装配。

  @req:ce10 @human
  场景: dispatch-via-shared
    - 产品 TUI 命令执行 MUST 委托 dispatch/Driver，MUST NOT 复制第二套语义。

  @req:ce12 @human
  场景: default-tui-entry
    - TTY 且无表面子命令时 MUST 默认进入产品 TUI。

  @req:ce13 @human
  场景: print-requires-prompt
    - print MUST 有非空 prompt；MUST NOT 使用 Hello! 占位。

  @req:ce-st1 @human
  场景: session-tree-kind-api
    - Driver MUST 暴露 SessionTreeKind 区分的 session_tree / travel_session_tree；至少 MessageHistory；未实现 kind MUST 明确错误。

  @req:ce14 @human
  场景: stable-session-id-on-run
    - InProcessDriver::run MUST 复用 bootstrap session_id，MUST NOT 每轮新建 Uuid 文件。

  @req:ce15 @human
  场景: tokenizer-cache-subcommands
    - CLI MUST 提供顶层动词 tokenizer（跨面 ops，MUST NOT 仅挂在 tui 下），叶子至少含 status、download、clean：status MUST 可报告缓存根与条目（及可选 --model 的 builtin/local/cached/missing/unmapped）；download MUST 为显式 opt-in（调用即同意；TTY 默认可二次确认，--yes 跳过），MUST 在确认摘要中告知 repo/file、HF 基址与落盘路径，MUST NOT 在估计热路径静默下载；无配置映射且无显式 owner/repo 时 MUST 失败并提示配置 tokenizer 字段或 CLI 形状，MUST NOT 猜测下载目标；clean MUST 支持 --all 或按 model/target 删除；上述子命令 MUST 早退且 MUST NOT 经 bootstrap 装配会话/MCP；实现 MUST 委托 bridge 缓存 API，MUST NOT 在 CLI 内直接发起 HTTP。

  @req:ce16 @human
  场景: cli-surface-vs-ops
    - CLI MUST 区分表面动词与管理动词：表面至少含 tui 与 print（xylitol tui 进入 TUI；xylitol print 进入一次性 print 且须非空 prompt）；管理动词 resources、serve、tokenizer MUST 保持顶层，MUST NOT 挪入 tui 子树；TTY 裸跑默认 TUI 的语义 MUST 保持；MUST NOT 提供顶层 --tui/--print/-p/--prompt 或位置 PROMPT 作为表面切换别名（print 的 --prompt/-p 仅允许挂在 print 子命令下）。

  @req:ce17 @human
  场景: config-load-fail-closed
    - load_app_config（或等价）失败（含模板渲染、YAML 解析、IO）时，bootstrap MUST 返回硬错误且经其装配的全表面 MUST 非零退出；MUST NOT 仅 Warning 后继续并用 env 默认模型进入 TUI/print。

  @req:ce18 @human
  场景: unset-model-display
    - 当前未选中模型时，产品面展示的模型名 MUST 为 NOT-SET（或文档化的等价明确占位）；MUST NOT 将未显式配置/未传 --model 的状态显示为 gpt-4o 等厂商默认模型名。

  @req:ce19 @human
  场景: surface-owned-flags
    - CLI MUST 将表面旗标挂在表面动词下：`xylitol tui`（含可选 `run`）MUST 接受 --session/--model/--list-models/--trust/--no-trust/--config/--no-color/--attach/--port；`xylitol print` MUST 接受 --session/--model/--config/--no-color/--trust/--no-trust 且 MUST NOT 接受 --list-models/--attach/--port；顶层 MUST NOT 再提供上述旗标（解析 MUST 失败）。print 的 --trust/--no-trust MUST 经 bootstrap trust_override 生效（interactive=false，MUST NOT 弹出 ChoicePrompt）。`tui --session` 与 `tui run --session` MUST 等价生效。TUI 的 --attach 指定 Host URL（默认 http://127.0.0.1:18790）；仅 --port 时 MUST 连 http://127.0.0.1:<port>；二者都给时 --attach 胜。

  @req:ce20 @human
  场景: resume-hint-on-exit
    - TUI 与 print 正常退出后，若当前 session_id 出现在 Driver list_sessions（或等价已持久化判定）中，CLI MUST 向 stderr 打印恰好一行 `Resume by $ xylitol tui --session <uuid>`；未持久化或无 session_id 时 MUST NOT 打印该行。

  @req:ce21 @human
  场景: tui-attach-fail-closed
    - 产品 TUI（含 TTY 默认进入 TUI 表面）在 attach 目标 Host 未在听时 MUST 非零退出，并提示用 xylitol serve 启动 Host。MUST NOT 静默改走同进程直握操作器。print 与库嵌入 MUST NOT 适用本条。
