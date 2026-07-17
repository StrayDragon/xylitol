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
    那么 报错指向配置

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
