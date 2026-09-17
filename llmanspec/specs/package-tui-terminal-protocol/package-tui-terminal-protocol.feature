# language: zh-CN
# capability: package-tui-terminal-protocol
# purpose: packages/xylitol-tui 终端协议协商（Kitty / modifyOtherKeys / OSC）。
# scope: packages/xylitol-tui/, tests/

功能: package-tui-terminal-protocol

  @req:r1654 @human
  场景: kitty-protocol-probe
    - 终端层 MUST 在启动时探测 Kitty 键盘协议支持：发送 Kitty 渐进增强查询（CSI >7u 请求 flags，再 CSI ?u 弹出当前状态，再 CSI c 作为 device-attributes 哨兵），并在短有界窗口内读取响应。探测为尽力而为（路线 A：crossterm 保留输入层）。非零 flags 响应时，terminal MUST 启用 Kitty 消歧模式（经协议开关全局生效）并禁用 modifyOtherKeys。零 flags、device-attributes 响应或窗口超时时，terminal MUST 回退到 enable_modify_other_keys()。

  @req:r1655 @human
  场景: kitty-flags-parser
    - 终端层 MUST 提供纯解析器 parse_kitty_flags(seq)，将 Kitty 协议响应（CSI ?Nu，N 为协商 flags）映射为非负整数，非匹配序列返回 None。解析器 MUST 区分零 flags 响应（终端理解查询但不支持 Kitty flags）与无响应（timeout）。该解析器可无真实终端单测（c405 layer 1）。

  @req:r1656 @human
  场景: modify-other-keys-fallback
    - Kitty 键盘协议不可用时，terminal MUST 管理 xterm modifyOtherKeys 资源作为回退：enable_modify_other_keys() 发送 CSI >4;2m（mode 2），disable_modify_other_keys() 发送 CSI >4;0m（reset）。Kitty 协议激活时 terminal MUST NOT 启用 modifyOtherKeys（Kitty 优先）。stop() 时，terminal MUST 重置其启用的协议（若已 push 则发送 CSI <u 弹出 Kitty 状态，再 CSI >4;0m 清除 modifyOtherKeys）。

  @req:r1657 @human
  场景: osc-title-and-progress
    - Terminal trait MUST 暴露 set_title(title)，发出 OSC 0;title BEL（或等价 OSC 2），以及 set_progress(active)，active 时发出 OSC 9;4;3，清除时 OSC 9;4;0（pi 使用的 xterm/conpty progress 约定）。VirtualTerminal 测试替身 MUST 记录这些调用（或 no-op）而不发出真实转义。与 pi 不同，xy 不为 progress 运行 1 秒 keepalive 间隔（TUI 在 stop 时清除 progress）；每次状态变化单次发射即可。

  @req:r1658 @human
  场景: color-scheme-pure-helpers
    - packages/xylitol-tui 的 terminal_colors MUST 提供纯函数：从 OSC11 背景 RGB 经相对亮度判定 Dark/Light、解析 COLORFGBG（bg>=7 为 Light）、以及按优先级合成多源探测结果；MUST NOT 要求真实 TTY 或事件循环即可单测。

  @req:r1659 @human
  场景: color-scheme-priority
    - 多源合成 MUST 按显式注入 > OSC11 亮度 > CSI 997 报告 > COLORFGBG > 默认 Dark 的优先级选择 TerminalColorScheme。

  @req:r1660 @human
  场景: mouse-capture-opt-in
    - Terminal（或 CrosstermTerminal 等价面）MUST 提供显式 enable_mouse_capture / disable_mouse_capture（或同名 API）；start / 默认构造路径 MUST NOT 自动 EnableMouseCapture。若曾 Enable，stop 与 finish_inline（或等价 teardown）MUST Disable。VirtualTerminal 测试替身 MUST 可记录启停调用序。本要求 MUST NOT 规定产品何时开启捕获。
