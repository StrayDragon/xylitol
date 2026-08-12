# language: zh-CN
功能: package-tui-interaction-modes

  @req:ptim01
  场景: default-mode-a-inline
    假如 新建默认 TUI
    当 查询交互模式
    那么 模式为 Inline 且应用会话未激活

  @req:ptim05
  场景: copy-on-release-osc52
    假如 ApplicationOwned 应用会话已 begin 且 transcript 有可拖选文本
    当 未修饰左键拖选非空范围并松开
    那么 发出 OSC52 剪贴板序列

  @req:ptim12
  场景: dock-drag-clamp-keeps-selection
    假如 transcript 拖选进行中
    当 指针拖入 dock 矩形
    那么 选区仍在且焦点夹在 transcript 底边

  @req:ptim10
  场景: wheel-sticky-viewport
    假如 ApplicationOwned 视口已 follow 到底
    当 在 transcript 内滚轮向上并重绘
    那么 scroll_top 不回到底部

  @req:ptim15
  场景: copy-notice-after-success
    假如 ApplicationOwned 应用会话已 begin
    当 松手复制成功
    那么 copy-notice 信号可观察且空选不发

  @req:ptim13
  场景: editor-multiline-selection
    假如 ApplicationOwned 下 Editor 有多行缓冲
    当 在 Editor 内未修饰拖选跨行并松开
    那么 仅输入缓冲文本进入复制路径且 transcript 选区未写入

  @req:ath30
  场景: product-default-mode-a
    假如 产品 TuiRunOptions 默认值
    当 读取 interaction_mode
    那么 为 Inline

  @req:ath31
  场景: product-copy-notice-wiring
    假如 产品 ApplicationOwned 会话可接收库 copy-notice
    当 读取 interaction_mode
    那么 短时提示路径存在且不使用 Error 前缀拒闸 toast 冒充成功
