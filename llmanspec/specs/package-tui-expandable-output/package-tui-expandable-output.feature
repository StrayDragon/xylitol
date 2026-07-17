# language: zh-CN
# managed by llman sdd partition-migrate
功能: package-tui-expandable-output

  @req:r29
  场景: shell-present
    当 列出 llmanspec/specs/package-tui-expandable-output
    那么 spec.toon 存在且 purpose 为中文

  @req:peo1
  场景: api-export
    当 从 xylitol_tui 引用
    那么 ExpandableOutput 与 render_expandable_output 可导出

  @req:peo2
  场景: hint-below-tail
    假如 文本超过 max_preview_lines
    当 collapsed Tail render
    那么 末行含 earlier lines 与 expand_hint 且首行是可见尾窗起点

  @req:peo3
  场景: stream-then-expand
    假如 连续 append 多行
    当 collapsed 再 set_expanded
    那么 collapsed 贴尾且 expanded 无 earlier 提示

  @req:peo0
  场景: shell-present
    当 列出 capability package-tui-expandable-output
    那么 delta 含 peo4–peo5

  @req:peo4
  场景: head-hint
    假如 文本超过 max_preview_lines 且 Head
    当 collapsed render
    那么 末行附近含 more lines 与 expand_hint 且首行是原文头

  @req:peo5
  场景: empty-text
    当 空串 render
    那么 不 panic 且行数有界

  @req:peo5
  场景: width-one
    假如 多行文本
    当 width=1 collapsed render
    那么 不 panic
