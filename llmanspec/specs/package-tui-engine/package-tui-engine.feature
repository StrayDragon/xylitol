# language: zh-CN
# managed by llman sdd partition-migrate
功能: package-tui-engine

  @req:pte01
  场景: container-concatenates-children
    假如 Container 持有 Text(a) 再 Text(b)
    当 render(width) 被调用
    那么 输出行按 a 再 b 顺序

  @req:pte01
  场景: container-does-not-fanout-input
    假如 Container 有 child Input
    当 Container.handle_input 收到 Key 事件
    那么 除非 host/TUI 直接聚焦该 child，否则 child Input 不收到事件

  @req:pte02
  场景: overlay-hide-restores-prefocus
    假如 capturing overlay 聚焦在 root editor 上
    当 OverlayHandle.hide
    那么 焦点回到 editor pre_focus

  @req:pte03
  场景: overlay-focus-nc-allowed
    假如 可见 non_capturing overlay
    当 OverlayHandle.focus
    那么 overlay 获焦并接收后续输入

  @req:pte03
  场景: overlay-unfocus-no-reclaim
    假如 capturing overlay 已聚焦
    当 unfocus 再 dispatch_event
    那么 root 保持焦点；overlay 不 reclaim

  @req:pte04
  场景: agent-demo-builds-without-image
    假如 xylitol-tui 包以默认 features 构建
    当 agent_demo 与 cargo test -p xylitol-tui 运行
    那么 成功且不依赖 Image 组件 API

  @req:pte05
  场景: listener-consumes-before-focus
    假如 已注册对 ctrl+c 返回 Consumed 的 listener，且 Editor 处于焦点
    当 调用 dispatch_event(Key(ctrl+c))
    那么 Editor 不收到该事件

  @req:pte05
  场景: listener-yields-to-focus
    假如 listener 对普通字符返回 Continue，Editor 处于焦点
    当 调用 dispatch_event(Key(a))
    那么 Editor 收到该事件

  @req:pte05
  场景: paste-through-listeners
    假如 listener 对 Paste 返回 Continue
    当 调用 dispatch_event(Paste(...))
    那么 焦点组件收到 paste

  @req:pte06
  场景: reclaim-after-set-focus-steal
    假如 capturing overlay 可见
    当 set_focus root 再 dispatch_event
    那么 overlay reclaim 焦点并接收事件

  @req:pte06
  场景: set-focus-none-clears
    假如 overlay 聚焦且 eligible restore
    当 set_focus None
    那么 restore inactive 且 overlay 不在下次输入 reclaim

  @req:pte06
  场景: nested-retarget-on-hide
    假如 parent 再 child capturing overlays
    当 hide child
    那么 焦点经 retargeted pre_focus 恢复到 parent
