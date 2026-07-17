# language: zh-CN
# managed by llman sdd partition-migrate
功能: package-tui-terminal-protocol

  @req:tp01
  场景: kitty-query-sent-at-start
    假如 CrosstermTerminal 在真实终端（c405 layer 5a PTY E2E）启动
    当 捕获 terminal 写入的字节流
    那么 含 Kitty 查询序列 CSI >7u

  @req:tp01
  场景: kitty-activated-on-flags-response
    假如 探测在窗口内收到 CSI ?7u 响应
    当 set_kitty_protocol_active 被调用
    那么 is_kitty_protocol_active() 返回 true 且 modifyOtherKeys 未启用

  @req:tp01
  场景: fallback-on-timeout
    假如 探测窗口超时且无 Kitty 响应
    当 terminal 回退
    那么 发送 enable_modify_other_keys（CSI >4;2m）且 is_kitty_protocol_active() 保持 false

  @req:tp02
  场景: parse-flags-nonzero
    假如 parse_kitty_flags 收到 CSI ?7u 序列
    当 调用解析器
    那么 返回 7

  @req:tp02
  场景: parse-flags-zero
    假如 parse_kitty_flags 收到 CSI ?0u 序列
    当 调用解析器
    那么 返回 0（与 None 区分）

  @req:tp02
  场景: parse-flags-nonmatch
    假如 parse_kitty_flags 收到 device-attributes 响应或无关序列
    当 调用解析器
    那么 返回 None

  @req:tp03
  场景: modify-enabled-only-when-no-kitty
    假如 terminal 在无 Kitty 支持下启动
    当 发送 modifyOtherKeys enable
    那么 写入流含 CSI >4;2m 且不含 Kitty pop

  @req:tp03
  场景: protocols-reset-on-stop
    假如 terminal 在启用任一协议后 stop
    当 stop() 被调用
    那么 写入流含 Kitty pop（若已 push）与 CSI >4;0m

  @req:tp04
  场景: title-set-via-osc
    假如 set_title 被调用并传入字符串
    当 写入 OSC 序列
    那么 捕获字节含 OSC 0;（title）BEL

  @req:tp04
  场景: progress-active-then-clear
    假如 set_progress(true) 再 set_progress(false) 被调用
    当 两者均写入
    那么 流含 OSC 9;4;3 再 OSC 9;4;0

  @req:tp05
  场景: osc11-light-bg
    假如 背景 RGB 近白
    当 scheme_from_background_rgb
    那么 返回 Light

  @req:tp05
  场景: colorfgbg-light
    假如 COLORFGBG=0;15
    当 parse_colorfgbg
    那么 返回 Light

  @req:tp05
  场景: colorfgbg-dark
    假如 COLORFGBG=15;0
    当 parse_colorfgbg
    那么 返回 Dark

  @req:tp06
  场景: priority-osc11-over-env
    假如 COLORFGBG 指示 Dark 且 OSC11 为亮背景
    当 resolve_terminal_color_scheme
    那么 返回 Light
