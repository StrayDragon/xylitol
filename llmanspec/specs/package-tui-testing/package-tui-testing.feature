# language: zh-CN
# managed by llman sdd partition-migrate
功能: package-tui-testing

  @req:tt01
  场景: five-layers-distinct
    假如 检查测试目录布局
    当 tests/support/ 含按键序列辅助、快照 fixture、Clock 抽象；tests/tui_e2e/ 含 portable-pty 与 tmux 驱动
    那么 五层均存在且各层职责互不重叠（无层重复另一层断言）

  @req:tt02
  场景: keys-helper-drives-editor
    假如 测试经泛化辅助挂载 Input 并对 abc-左箭头-backspace 序列调用 keys
    当 按键序列经 handle_input 分发
    那么 组件状态反映 ab（输入 abc、左移光标、backspace 删 b）且未重建第二实例

  @req:tt03
  场景: snapshot-detects-layout-regression
    假如 TUI 组件渲染输出变更布局（如面板边框样式）
    当 测试运行并调用 cargo insta review
    那么 快照 diff 呈现布局变更供人工审查

  @req:tt04
  场景: paste-burst-window-boundary
    假如 8ms 字符间阈值的 paste-burst 检测器以 mock clock 测试
    当 两字符间隔 7ms 注入再 9ms 注入
    那么 7ms 对被检测为 burst（enter 抑制）且 9ms 对不是（enter 提交），确定性且无 thread::sleep

  @req:tt05
  场景: spawn-primary-example
    假如 PTY E2E 层被要求 spawn agent_demo
    当 子进程在 portable-pty 下启动
    那么 请求的 example 启动且可用通用 send_keys 与 wait_for 辅助驱动

  @req:tt06
  场景: tmux-captures-colored-screen
    假如 xylitol 二进制在 TERM=xterm-256color 的 tmux 会话运行且渲染含红色词的回复
    当 调用 tmux capture-pane -e -p
    那么 捕获输出含 SGR 红色转义序列（CSI 31 m 或等价）确认真终端颜色渲染

  @req:tt07
  场景: agent-demo-submit-flow
    假如 假 coding-agent example 在代表性终端宽度进程内挂载
    当 测试提交默认 editor 内容
    那么 渲染成功无 RenderError 且 tool-running 状态可见

  @req:tt07
  场景: agent-demo-pty-smoke
    假如 假 coding-agent example 在 portable-pty 下运行
    当 经真实终端输入驱动提交流程
    那么 进程保持存活且提交后屏幕仍有内容
