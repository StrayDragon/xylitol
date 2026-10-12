# language: zh-CN
# capability: package-tui-tabs
# purpose: 通用标题条：键切下标，不持有页内容，暴露可见标题列区间。
# scope: src/, tests/

功能: package-tui-tabs

  @req:r1938
  规则: tabs-select-by-key
    标题条 须 维护一组标题与一个选中下标。收到已绑定的上一标题键 须 循环到前一个下标；收到已绑定的下一标题键 须 循环到后一个下标。未绑定这些键时，方向键与数字键 不得 改变下标。标题为空时已绑定的键 不得 改变状态。

    场景: tabs-bound-keys-cycle
      假如 有三个标题且选中第二个并且上一标题键与下一标题键已绑定
      当 依次收到下一标题键与上一标题键
      那么 选中下标先到第三个再回到第二个

    场景: tabs-unbound-keys-ignore
      假如 有三个标题且选中第二个并且上一标题键与下一标题键未绑定
      当 收到左方向键
      那么 选中下标仍是第二个
  @req:r1939
  规则: tabs-strip-only
    标题条渲染 须 只输出标题行。不得 持有或绘制与标题对齐的页内容。条下方画什么由调用方决定。

    场景: tabs-render-is-strip-only
      假如 有两个标题
      当 按固定宽度渲染
      那么 输出只有标题行且不含第二行正文
  @req:r1940
  规则: tabs-title-col-ranges
    按给定宽度渲染后 须 能读出每个可见标题在标题条上的列区间。每段 须 带该标题在完整标题表中的原下标。各区间 须 落在该宽度内、互不重叠、且对应可见标题文本。隐藏标题 不得 出现在区间表里。不得 登记命中区。

    场景: tabs-title-ranges-after-render
      假如 有两个标题
      当 按固定宽度渲染
      那么 每个可见标题有一段带原下标且落在宽度内、互不重叠的列区间

    场景: tabs-overflow-ranges-keep-source-index
      假如 有多个长标题且选中靠后的一个
      当 按不足以放下全部标题的宽度渲染
      那么 区间表每段的下标是原标题下标且不含隐藏标题
  @req:r1941
  规则: tabs-overflow-window
    标题合计超过给定宽度时 须 少画完整标题、用两侧计数标出隐藏数量，不得 把每个标题都截一段来塞满。当前选中标题 须 出现在条上且按自然宽完整可见（仅当这一条单独超过宽度时才截这一条）。隐藏标题 不得 出现在输出或列区间里。

    场景: tabs-overflow-keeps-selected
      假如 有多个长标题且选中其中一个
      当 按不足以放下全部标题的宽度渲染
      那么 输出含当前标题全文、不含全部隐藏标题、且两侧有隐藏计数
  @req:r1942
  规则: tabs-overflow-marker-ranges
    渲染后若某一侧有隐藏标题，须 能读出该侧计数标记的列区间，且 须 带该侧第一个隐藏标题的原下标。没有隐藏的一侧 不得 出现在标记区间表里。标记区间 不得 与可见标题区间重叠。不得 登记命中区。

    场景: tabs-overflow-marker-points-first-hidden
      假如 有多个长标题且选中靠中间的一个
      当 按不足以放下全部标题的宽度渲染
      那么 有隐藏的每一侧各有一段标记区间且下标是该侧第一个隐藏标题
  @req:r1943
  规则: tabs-click-selects
    收到左键按下且列落在上次渲染的可见标题区间内时，须 把选中下标改成该标题的原下标。列落在溢出标记区间内时，须 改成该侧第一个隐藏标题的原下标。列未命中或尚未渲染时 不得 改变下标。不得 登记命中区。

    场景: tabs-click-title-selects
      假如 有三个标题且选中第一个并已按固定宽度渲染
      当 在第二个标题的列上左键按下
      那么 选中下标变为第二个

    场景: tabs-click-overflow-jumps-hidden
      假如 有多个长标题且选中靠中间的一个并已按不足宽度渲染
      当 在右侧溢出标记的列上左键按下
      那么 选中下标变为右侧第一个隐藏标题
  @req:r1944
  规则: tabs-wheel-cycles
    收到滚轮向上 须 循环到下一个下标；收到滚轮向下 须 循环到上一个下标。标题为空或尚未渲染时 不得 改变下标。

    场景: tabs-wheel-up-selects-next
      假如 有三个标题且选中第二个并已渲染
      当 收到滚轮向上
      那么 选中下标变为第三个

    场景: tabs-wheel-down-selects-previous
      假如 有三个标题且选中第二个并已渲染
      当 收到滚轮向下
      那么 选中下标变为第一个
  @req:r1945
  规则: tabs-offer-pointer-screen-origin
    标题条 须 接受屏上原点。offer_pointer 或 handle_input 收到指针时 须 用屏上列减原点列、行减原点行后再对照区间。指针行不在条上 不得 改变下标且 offer_pointer 须 返回 false。命中并改下标时 offer_pointer 须 返回 true。尚未设置原点时 须 按 (0, 0) 解释。

    场景: tabs-pointer-off-strip-ignored
      假如 有三个标题且原点在第 0 行并已渲染
      当 在第 2 行左键按下
      那么 选中下标不变且 offer_pointer 为假

    场景: tabs-pointer-on-strip-consumes
      假如 有三个标题且选中第一个、原点在第 0 行并已渲染
      当 在第 0 行且落在第二个标题列上左键按下
      那么 选中下标变为第二个且 offer_pointer 为真
