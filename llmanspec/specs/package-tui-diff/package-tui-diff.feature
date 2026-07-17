# language: zh-CN
# managed by llman sdd partition-migrate
功能: package-tui-diff

  @req:r28
  场景: shell-present
    当 列出 llmanspec/specs/package-tui-diff
    那么 spec.toon 存在且 purpose 为中文

  @req:ptd1
  场景: render-line-pair
    当 DiffInput::LinePair 含一行删除与一行添加
    那么 render 输出含着色的 -/+ 行

  @req:ptd1
  场景: empty-diff
    当 old 与 new 相同
    那么 输出含 (no changes) 或空等价提示

  @req:ptd2
  场景: intra-line-pair
    当 相邻单行 - 与 + 仅一词不同
    那么 变更词带 reverse/word_change

  @req:ptd3
  场景: narrow-unified
    当 width 小于 side_by_side 阈值且启用 side-by-side 选项
    那么 仍渲染 unified 单栏

  @req:ptd3
  场景: cjk-width
    当 含全角字符的 diff 行
    那么 折行/截断按 visible_width 不按字节

  @req:ptd4
  场景: parse-edit-line
    当 EditText 含 + 12 fn foo()
    那么 unified 输出含紧凑 + 与行号 12

  @req:ptd4
  场景: edit-pad-width
    当 EditText 行号含 9 与 100
    那么 行号列宽至少 3

  @req:ptd5
  场景: sbs-shows-line-nos
    当 LinePair 启用 side-by-side 且 width 足够
    那么 左右栏输出含对应行号数字

  @req:ptd5
  场景: sbs-empty-half
    当 仅删除行无配对添加
    那么 右半栏空白且无伪造行号

  @req:ptd6
  场景: highlight-identity
    当 未设置自定义 highlight_line
    那么 内容仍按 added/removed/context 着色且可渲染

  @req:ptd7
  场景: sbs-no-row-bg
    假如 theme 配置了非 identity 的 added_line_bg
    当 width 达到 side_by_side 阈值并渲染 SBS
    那么 输出半栏行不含该行底 SGR；unified 同 theme 仍可含行底

  @req:ptd0
  场景: shell-present
    当 列出 capability package-tui-diff
    那么 delta 含 ptd8–ptd9

  @req:ptd8
  场景: cjk-wrap
    假如 含全角字符的增删行
    当 在窄 width 下 render
    那么 不出现半个汉字且布局按 visible_width

  @req:ptd9
  场景: empty-right
    假如 仅删除行无配对添加
    当 启用 side-by-side 且宽度足够
    那么 右半栏无行号数字且左半栏有 old 行号

  @req:ptd9
  场景: empty-left
    假如 仅添加行无配对删除
    当 启用 side-by-side 且宽度足够
    那么 左半栏无伪造行号
