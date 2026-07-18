# language: zh-CN
# managed by llman sdd partition-migrate
功能: package-tui-theme

  @req:ptt01
  场景: dark-light
    当 调用 SemanticPalette::dark 与 light
    那么 两套 on_surface 等 token 不同

  @req:ptt01
  场景: for-scheme
    当 for_scheme(Light)
    那么 得到与 light 等价色板

  @req:ptt02
  场景: fg-reset
    当 fg_rgb 包装字符串
    那么 输出含 38;2 与 39m

  @req:ptt03
  场景: md-factory
    假如 给定 Dark palette
    当 markdown_theme
    那么 返回可用 MarkdownTheme

  @req:ptt04
  场景: demo-light
    假如 theme_auto 且探测为 Light
    当 渲染 footer 与 md/diff chrome
    那么 可见 theme:light 且 chrome 非 Mocha 硬编码

  @req:ptt05
  场景: default-dark
    假如 未设 THEME_AUTO
    当 启动 demo
    那么 theme_mode 为 Dark

  @req:ptt06
  场景: feed-osc
    假如 auto 开启
    当 feed OSC11 浅色背景 reply
    那么 theme_mode 变为 Light

  @req:ptt06
  场景: query-consts
    当 读取查询常量
    那么 含 OSC 11 ? 与 CSI 996

  @req:ptt07
  场景: levels-cycle
    当 从 Off 连续 cycle_next 七次
    那么 回到 Off 且途经 Max

  @req:ptt07
  场景: rgb-distinct
    假如 同一 Palette
    当 比较 medium 与 high 的 thinking_border_rgb
    那么 两色不等

  @req:ptt07
  场景: paint-truecolor
    假如 给定 Dark palette 与 High
    当 调用 thinking_border_paint 包装文本
    那么 输出含 38;2 真彩 SGR

  @req:ptt07
  场景: pi-dark-high
    当 Dark palette thinking_border_rgb(High)
    那么 等于 pi #b294bb
