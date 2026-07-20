# language: zh-CN
# managed by llman sdd partition-migrate
功能: cli-entry

  @req:r67
  场景: fields
    假如 skill 命令注册
    当 查询 SlashCommandInfo
    那么 source 为 Skill 且 source_info 有值

  @req:r68
  场景: source-values
    假如 枚举来源
    当 检查
    那么 仅为 prompt 或 skill

  @req:r69
  场景: getcommands-matches-catalog
    假如 InProcessDriver 与产品 SlashCommandSource
    当 列出命令名
    那么 集合一致（允许 quit 仅作 exit 解析别名）

  @req:r69
  场景: no-legacy-short-names
    假如 GetCommands 返回列表
    当 检查名称
    那么 不含 tree/fork/export/compact/resume/new 作为产品主名

  @req:r70
  场景: dispatch-export
    假如 用户 /session-export
    当 经 effects
    那么 导出路径执行

  @req:ce1
  场景: cli-under-app
    假如 检查源码树
    当 src/app/cli
    那么 存在且无 src/app/print 顶级模块

  @req:ce2
  场景: typo-error
    假如 配置 models 因 typo 为空
    当 启动 CLI
    那么 报错指向配置且 MUST NOT 静默进入 TUI

  @req:ce17
  场景: config-template-fail-closed
    假如 项目 config.yaml 因未定义的模板变量导致加载失败
    当 经 bootstrap 启动 TUI 或 print 或 --list-models
    那么 硬失败非零退出且 MUST NOT 进入 TUI

  @req:ce18
  场景: unset-model-shows-not-set
    假如 无配置模型且未传 --model
    当 查询当前选中模型展示名
    那么 为 NOT-SET 而非 gpt-4o

  @req:ce6
  场景: same-handler-both-drivers
    假如 同一 handler
    当 InProcess 与 Remote
    那么 事件序列同构

  @req:ce7
  场景: cli-constructs-ports
    假如 追踪装配
    当 注入
    那么 含 SessionStore 与 EventSink

  @req:ce8
  场景: server-stop-signals
    假如 server 有 lock PID
    当 xylitol server stop
    那么 SIGTERM 该 PID

  @req:ce9
  场景: server-bootstrap
    假如 cwd 有 AGENTS.md
    当 server 启动
    那么 经 bootstrap 注入 context

  @req:ce10
  场景: tui-model-works
    假如 TUI 运行
    当 /model id
    那么 经 dispatch 切换

  @req:ce11
  场景: rpc-files-gone
    假如 检查树
    当 rpc.rs 与 --rpc
    那么 均不存在

  @req:ce12
  场景: bare-tty-tui
    假如 TTY 无 prompt
    当 运行 xylitol
    那么 进入 TUI

  @req:ce13
  场景: no-hello-fallback
    假如 非 TTY 无 prompt
    当 运行
    那么 错误退出且无 Hello!

  @req:ce-st1
  场景: message-history-tree
    假如 session 有消息
    当 session_tree(MessageHistory)
    那么 非空树

  @req:ce14
  场景: stable-id
    假如 同一 Driver 两次 run
    当 session_id
    那么 保持不变

  @req:ce15
  场景: tokenizer-help-tree
    假如 CLI 已解析
    当 xylitol tokenizer --help
    那么 可见 status、download、clean 叶子

  @req:ce15
  场景: tokenizer-status-empty
    假如 缓存目录为空
    当 xylitol tokenizer status
    那么 报告缓存根且不失败伪装已下载

  @req:ce15
  场景: tokenizer-download-opt-in
    假如 目标映射到 HuggingFace 且本地无缓存
    当 xylitol tokenizer download <target> --yes
    那么 词表落入缓存路径且再次 status 可见

  @req:ce15
  场景: tokenizer-clean
    假如 缓存中已有条目
    当 xylitol tokenizer clean --all
    那么 条目被移除且 status 不再列出

  @req:ce15
  场景: tokenizer-no-bootstrap
    假如 仅执行 tokenizer 子命令
    当 xylitol tokenizer status
    那么 不经 bootstrap 装配会话或 MCP 即可完成

  @req:ce15
  场景: tokenizer-download-shows-hf-base
    假如 已设置 HF_ENDPOINT 为镜像基址且目标已映射
    当 xylitol tokenizer download <target> 进入确认摘要（或 --yes 的等价日志）
    那么 摘要含该镜像基址与落盘路径

  @req:ce16
  场景: surface-tui-verb
    假如 TTY
    当 xylitol tui
    那么 进入产品 TUI

  @req:ce16
  场景: surface-print-verb
    假如 无 prompt
    当 xylitol print
    那么 错误退出且无 Hello!

  @req:ce16
  场景: ops-stay-toplevel
    假如 CLI 已解析
    当 xylitol --help
    那么 Commands 含 tokenizer 与 resources 为顶层而非 tui 子命令
