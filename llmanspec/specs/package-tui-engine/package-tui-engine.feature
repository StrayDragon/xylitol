# language: zh-CN
# capability: package-tui-engine
# purpose: packages/xylitol-tui 核心 TUI 引擎 API（Container、OverlayHandle、可选 image surface）。
# scope: packages/xylitol-tui/, tests/

功能: package-tui-engine

  @req:r1614 @human
  场景: container-vertical-stack
    - xylitol-tui 包 MUST 提供 Container 组件，持有有序子 Component 列表，按顺序拼接各 child 的 render(width) 行渲染，invalidate 时使所有 child 失效，MUST NOT 自动将 InputEvent fan-out 到 child（焦点路由仍为 TUI/host 职责）。Container MUST 暴露 add_child、remove_child 与 clear。

  @req:r1615 @human
  场景: overlay-handle
    - TUI::show_overlay MUST 返回 OverlayHandle，可 hide（永久移除）、set_hidden / is_hidden（临时）、report is_focused。hide MUST 经 focus-restore 状态机恢复焦点（最上层可见 capturing overlay，否则 retarget 后的 pre_focus FocusTarget）。handle MUST 仅在 overlay 条目存在时可用（hide 后操作为 no-op）。

  @req:r1616 @human
  场景: overlay-handle-focus-controls
    - OverlayHandle MUST 暴露 focus 与 unfocus 操作。focus MUST 将可见 overlay（含 non_capturing）带到视觉最前并捕获输入。unfocus MUST 释放焦点而不 rebuild；unfocus MAY 接受显式 FocusTarget。TUI MUST 实现 eligible/blocked/resume overlay focus-restore，并在焦点不在 overlay 上时于 dispatch_event reclaim。

  @req:r1617 @human
  场景: image-surface-optional
    - 包 MUST NOT 要求 Image 组件或完整 terminal_image encode 路径用于主 agent_demo / app-shell 接线。is_image_line（及 width invariant 所需的 minimal helpers）MUST 保留。未使用的 image encode/UI surface MAY 移除或 feature-gate。

  @req:r1618 @human
  场景: input-listener-pre-focus
    - TUI MUST 提供 add_input_listener / remove_input_listener；每个 InputEvent 在路由到 focused overlay 或 focused root child 之前 MUST 按注册顺序调用 listener；任一 listener 返回 Consumed MUST 阻止后续 listener 与焦点组件接收该事件。

  @req:r1619 @human
  场景: overlay-focus-restore-sm
    - TUI MUST 维护 overlay focus-restore 状态 inactive/eligible/blocked；capturing overlay 获焦时 MUST 进入 eligible；焦点被临时转到非祖先 root/child 时 MUST 进入 blocked；dispatch_event 在 listener 之后 MUST reclaim eligible 或满足条件的 blocked resume；set_focus(None) 在 clear 策略下 MUST 清除 restore；non_capturing show 与 set_hidden(false) MUST NOT 自动抢焦点。

  @req:r1620 @human
  场景: input-event-mouse
    - InputEvent MUST 包含 Mouse 变体（携带列/行与按键或移动类别，足以表达 crossterm MouseEvent 语义）；dispatch_event 与 add_input_listener MUST 可接收 Mouse；demo/start 事件环 MUST 能将 crossterm Event::Mouse 映射为 InputEvent::Mouse。默认路径下 Mouse Moved（或等价无态变移动）MUST NOT 强制整帧 do_render / 递增 frame_count；本要求 MUST NOT 规定产品折叠点击语义。
