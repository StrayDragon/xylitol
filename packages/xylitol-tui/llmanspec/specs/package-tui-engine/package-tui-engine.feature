# language: zh-CN
# capability: package-tui-engine
# purpose: packages/xylitol-tui 核心 TUI 引擎 API（Container、OverlayHandle、可选 image surface）。
# scope: src/, tests/

功能: package-tui-engine

  @req:r1614
  规则: container-vertical-stack
    xylitol-tui 包 MUST 提供 Container 组件，持有有序子 Component 列表，按顺序拼接各 child 的 render(width) 行渲染，invalidate 时使所有 child 失效。handle_input MUST NOT 把键盘或粘贴转给 child（焦点路由仍为 TUI/host 职责）。Container MUST 暴露 add_child、remove_child 与 clear。
    # verified-by: packages/xylitol-tui/tests/tui_integration_test.rs
  @req:r1615
  规则: overlay-handle
    TUI::show_overlay MUST 返回 OverlayHandle，可 hide（永久移除）、set_hidden / is_hidden（临时）、report is_focused。hide MUST 经 focus-restore 状态机恢复焦点（最上层可见 capturing overlay，否则 retarget 后的 pre_focus FocusTarget）。handle MUST 仅在 overlay 条目存在时可用（hide 后操作为 no-op）。
    # verified-by: packages/xylitol-tui/tests/overlay_focus_test.rs
  @req:r1616
  规则: overlay-handle-focus-controls
    OverlayHandle MUST 暴露 focus 与 unfocus 操作。focus MUST 将可见 overlay（含 non_capturing）带到视觉最前并捕获输入。unfocus MUST 释放焦点而不 rebuild；unfocus MAY 接受显式 FocusTarget。TUI MUST 实现 eligible/blocked/resume overlay focus-restore，并在焦点不在 overlay 上时于 dispatch_event reclaim。
    # verified-by: packages/xylitol-tui/tests/overlay_focus_test.rs
  @req:r1617
  规则: image-surface-optional
    包 MUST NOT 要求 Image 组件或完整 terminal_image encode 路径用于主 agent_demo / app-shell 接线。is_image_line（及 width invariant 所需的 minimal helpers）MUST 保留。未使用的 image encode/UI surface MAY 移除或 feature-gate。
    # verified-by: packages/xylitol-tui/src/terminal_image.rs
  @req:r1618
  规则: input-listener-pre-focus
    TUI MUST 提供 add_input_listener / remove_input_listener；每个 InputEvent 在路由到 focused overlay 或 focused root child 之前 MUST 按注册顺序调用 listener；任一 listener 返回 Consumed MUST 阻止后续 listener 与焦点组件接收该事件。
    # verified-by: packages/xylitol-tui/tests/tui_integration_test.rs
  @req:r1619
  规则: overlay-focus-restore-sm
    TUI MUST 维护 overlay focus-restore 状态 inactive/eligible/blocked；capturing overlay 获焦时 MUST 进入 eligible；焦点被临时转到非祖先 root/child 时 MUST 进入 blocked；dispatch_event 在 listener 之后 MUST reclaim eligible 或满足条件的 blocked resume；set_focus(None) 在 clear 策略下 MUST 清除 restore；non_capturing show 与 set_hidden(false) MUST NOT 自动抢焦点。
    # verified-by: packages/xylitol-tui/tests/overlay_focus_test.rs
  @req:r1620
  规则: input-event-mouse
    InputEvent MUST 包含 Mouse 变体（携带列/行与按键或移动类别，足以表达 crossterm MouseEvent 语义）；dispatch_event 与 add_input_listener MUST 可接收 Mouse；demo/start 事件环 MUST 能将 crossterm Event::Mouse 映射为 InputEvent::Mouse。    默认路径下 Mouse Moved（或等价无态变移动）MUST NOT 强制整帧 do_render / 递增 frame_count；本要求 MUST NOT 规定产品折叠点击语义。
  @req:r1946
  规则: pointer-offer-before-focus
    Component MUST 提供默认空操作的 set_screen_origin 与 offer_pointer。offer_pointer 收到指针且组件认领时 MUST 返回 true，未认领 MUST 返回 false。TUI::dispatch_event 在 listener 之后、焦点 overlay 或焦点根 child 之前 MUST 对可见 overlay 与根 child 调用 offer_pointer；任一返回 true MUST 阻止焦点组件再收到该指针事件。Container 的 offer_pointer MUST 按上次 render 各 child 占用的行把屏上事件转给命中的 child。键盘与粘贴 MUST NOT 经 offer_pointer 转发。Moved 被 offer_pointer 拒绝时 MUST NOT 仅因此强制整帧重绘。

    场景: dispatch-offer-pointer-blocks-focus
      假如 根 child 会认领指针且另一组件持有焦点
      当 dispatch_event 收到该指针
      那么 焦点组件收不到该事件
    # verified-by: packages/xylitol-tui/tests/tui_integration_test.rs
  @req:r1954
  规则: viewport-overlay-after-clip
    xylitol-tui MUST 在可见帧拼好之后做视口叠层：ApplicationOwned 为 project_frame 与 reproject_frame 之后（含只投影的滚轮帧）；Inline 为 Component::render 之后。视口叠层 MUST 先按屏格 composite 已登记 overlay（锚点相对终端高宽，MUST NOT blit 到未裁切内容缓冲），再对每个根 Component 调用默认空操作的 overlay_viewport(&self, lines, width, transcript_rows)。overlay_viewport MUST 只改已裁切的 lines；transcript_rows MUST 为可见 transcript 行数（不含 dock）；默认实现 MUST 为 no-op。MUST NOT 为视口装饰把组件树标为整段 stale。库 copy-notice 仍只发信号（r1640），MUST NOT 在引擎内画 Copied 文案。
    场景: overlay-viewport-pins-after-reproject
      假如 ApplicationOwned 已 ingest 长内容且根组件 overlay_viewport 会在可见顶行写标记
      当 只 reproject_frame 且 scroll_top 大于 0
      那么 可见帧顶行含该标记且 ScrollView 内容第 0 行不含
    # verified-by: packages/xylitol-tui/tests/tui_integration_test.rs
# re-review(c2826): 复审结论——本 capability 管辖行为不变；分支内改动仅测试基建与可见性再导出（2026-09-28）

# re-review(c2827): 复审结论——本 capability 管辖行为不变；分支内改动为 BDD 场景落地、BDD 测试基建（steps/bindings/驱动旋钮与探针）与可见性再导出（2026-09-28）
    # verified-by: packages/xylitol-tui/tests/tui_integration_test.rs
