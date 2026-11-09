# language: zh-CN
# purpose: 通用有界滚动区域组件：定高视口、按宽换行、滚动条、键盘/滚轮/gutter 交互。

功能: package-tui-scroll-region

  @req:r3280
  规则: bounded-viewport-render
    ScrollRegion 须 提供定高视口：内容按宽度换行后只显示视口窗口；内容不足视口高度时 须 补空行到恰好视口高度；width 为 0 时 须 安全返回；不得 panic。
    内容行数超过视口高度时 须 可滚动；不得把内容截断为单行或撑高视口。

    场景: 长行换行且只显示视口窗口
      假如 ScrollRegion 视口高度为 3 且内容含一条超过宽度的长行
      当 以宽度 20 渲染
      那么 输出恰好 3 行且长行按宽度换行后只显示前 3 个视觉行

    场景: 内容不足补空行
      假如 ScrollRegion 视口高度为 5 且内容只有 2 行
      当 渲染
      那么 输出恰好 5 行且末 3 行为空行

  @req:r3281
  规则: scroll-navigation
    ScrollRegion 须 支持按视口高度滚动：PageUp/PageDown 每次滚动一个视口高度；滚轮每次滚动一个 wheel notch；滚动位置 须 clamp 到 0..=max_scroll；内容不超过视口高度时 不得 改变滚动位置。
    滚动 不得 依赖 transcript 视口状态。

    场景: PageDown 滚动一页
      假如 内容行数大于视口高度且滚动位置为 0
      当 按下 PageDown
      那么 滚动位置增加一个视口高度且不超过 max_scroll

    场景: 内容不足时滚轮不动
      假如 内容行数不超过视口高度
      当 滚轮向下
      那么 滚动位置保持 0

  @req:r3282
  规则: overflow-gutter
    当 内容视觉行数超过视口高度且宽度不小于 gutter 最小宽度时，ScrollRegion 须 在最右 1 列绘制 gutter：顶行 `^`、底行 `v`、内域按滚动比例绘制 thumb；内容不超过视口高度或宽度不足时 不得 绘制 gutter 且 不得 预留列。
    点击 gutter 顶行 须 滚动到顶；点击底行 须 滚动到底；轨道点击 不得 产生动作。

    场景: 溢出时绘制 gutter
      假如 内容视觉行数超过视口高度且宽度足够
      当 渲染
      那么 最右列为 gutter 且正文按少一列排版

    场景: 无溢出时隐藏 gutter
      假如 内容视觉行数不超过视口高度
      当 渲染
      那么 不绘制 gutter 且正文占满宽度

  @req:r3283
  规则: pointer-scope
    ScrollRegion 的指针命中 须 只认领视口矩形内的滚轮与 gutter 点击；视口外事件 须 返回未认领；不得 启动 transcript 拖选或认领槽外事件。
    命中 须 按屏上实际矩形，不得 按冻结坐标。

    场景: 视口外滚轮不认领
      假如 ScrollRegion 已渲染在屏幕矩形内
      当 在矩形外收到滚轮
      那么 offer_pointer 返回 false

    场景: 视口内滚轮认领
      假如 ScrollRegion 内容超过视口高度
      当 在矩形内收到滚轮向下
      那么 offer_pointer 返回 true 且滚动位置增加一个 notch
