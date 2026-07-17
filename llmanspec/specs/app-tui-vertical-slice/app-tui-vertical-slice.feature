# language: zh-CN
# managed by llman sdd partition-migrate
功能: app-tui-vertical-slice

  @req:avs1
  场景: h2-stream
    假如 HostSession 已 on_run_started
    当 注入 TextDelta 与 AgentEnd
    那么 scrollback 含助手文本且 phase 为 Idle

  @req:avs1
  场景: h3-tool
    假如 Busy 中
    当 注入 ToolExecutionStart/End
    那么 存在 UiEntry::Tool 且渲染含工具名

  @req:avs1
  场景: h4-steer
    假如 Busy 且 editor 非空
    当 薄编排处理 Enter
    那么 Driver::steer 被记录且出现 Steering: strip

  @req:avs1
  场景: h7-abort-resume
    假如 Busy 中 Esc 已 abort
    当 再 idle 提交
    那么 第二次 Driver::run 被调用且无粘性 aborted

  @req:avs1
  场景: h8-exit
    假如 idle 输入 /exit
    当 编排收尾
    那么 session quit 且 finish_inline/stop 可观测

  @req:avs2
  场景: pty-hello-exit
    假如 PTY 下 Fake+--trust 产品 TUI 已就绪
    当 提交短 prompt 再 /exit
    那么 屏含 Hello from fake provider 且进程退出
