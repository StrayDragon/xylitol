# language: zh-CN
# capability: package-tui-expandable-output
# purpose: 通用 ExpandableOutput：max-height 视口、earlier/more 提示、流式贴尾（供工具/bash 详情）。
# scope: packages/xylitol-tui/

功能: package-tui-expandable-output

  @req:peo1 @human
  场景: expandable-api
    - 包 MUST 提供 ExpandableOutput 与 render_expandable_output；选项 MUST 含 max_preview_lines、TruncateFrom（Tail 或 Head）、expand_hint、fold_hint；MUST NOT 依赖主 crate xylitol。

  @req:peo2 @human
  场景: collapsed-tail-hint
    - 当 collapsed 且跳过行数大于 0 时 Tail 与 Head 模式 MUST 均在可见窗口之下（块尾）渲染 dim 提示（Tail：earlier lines；Head：more lines；均含 expand_hint）；MUST NOT 把提示插在工具头行/首条可见正文之上。

  @req:peo3 @human
  场景: streaming-stick-tail
    - 流式追加文本时 collapsed Tail 视口 MUST 继续显示最新视觉行；expanded MUST 显示全文且无 earlier 提示；expanded 且内容视觉行数超过 max_preview_lines 时 MUST 在块尾渲染 dim 折叠提示行（`... (expanded, {fold_hint})`），内容未超上限或文本为空时 MUST NOT 渲染该行。

  @req:peo4 @human
  场景: head-more-hint
    - 当 TruncateFrom::Head 且 collapsed 并跳过行数大于 0 时，MUST 在可见首部之下渲染 dim 的 more lines 提示（含 expand_hint）；MUST NOT 把该提示放在首部之上冒充 Tail。

  @req:peo5 @human
  场景: zero-width-safe
    - ExpandableOutput 在 width 为 0 或 1、或文本为空时 MUST 安全返回（空或有界行）；MUST NOT panic。
