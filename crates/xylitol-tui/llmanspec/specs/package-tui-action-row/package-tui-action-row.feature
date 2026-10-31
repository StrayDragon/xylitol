# language: zh-CN
# capability: package-tui-action-row
# scope: src/components/action_row.rs
# purpose: 单行动作 chip 条：按标签渲染一串可点 chip，暴露每 chip 的列区间与激活/禁用态，不持有业务语义。

功能: package-tui-action-row

  @req:r2895
  规则: action-row-render-and-ranges
    ActionRow 必须 接受有序 chip 列表（每项含标签、是否激活、是否禁用）并按固定宽度渲染为单行。渲染后 必须 能读回每 chip 的列区间与该 chip 在完整列表中的下标；区间 必须 落在宽度内且互不重叠。激活 chip 必须 带可视区分（反色或等价）；禁用 chip 必须 不写入区间表。不得 因 chip 而换行或增高；宽度不足时 必须 从尾部整 chip 丢弃，不得 截半显示。

    场景: 区间落在宽度内且互不重叠
      假如 三个 chip 且宽度足够
      当 渲染动作条
      那么 输出只有一行且三个区间落在宽度内且互不重叠

    场景: 禁用 chip 不进区间表
      假如 中间一个 chip 被标为禁用
      当 渲染动作条并读取区间表
      那么 区间表不含该 chip 且可见标签也不含其文案

    场景: 宽度不足从尾丢弃整 chip
      假如 多个 chip 且宽度只够前面部分
      当 渲染动作条
      那么 输出仍是单行且被丢弃的 chip 不出现在区间表与可见文本中
