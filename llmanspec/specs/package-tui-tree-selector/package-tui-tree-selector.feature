# language: zh-CN
# capability: package-tui-tree-selector
# purpose: 通用 TreeSelector：flatten/过滤/搜索/翻页/fold/annotation（供双 Esc 会话树）。
# scope: packages/xylitol-tui/

功能: package-tui-tree-selector

  @req:pts1 @human
  场景: tree-node-model
    - 包 MUST 提供 TreeNode（id/label/children）与可导航 TreeSelector 组件；MUST NOT 依赖主 crate xylitol 或 session 类型。

  @req:pts2 @human
  场景: include-node-filter
    - TreeSelector MUST 支持 include_node 钩子过滤可见节点，并在过滤后重算 indent/connector/gutter；MUST NOT 硬编码产品 FilterMode 枚举。

  @req:pts3 @human
  场景: select-keybindings
    - TreeSelector 导航 MUST 使用 tui.select.* 键绑定（up/down/page/confirm/cancel）；选中态 MUST 可用主题闭包（如 reverse）表达。

  @req:pts4 @human
  场景: incremental-search
    - TreeSelector MUST 支持对可见节点 label 的增量搜索（可打印字符追加、Backspace 删除）；有搜索串时 Esc MUST 先清空搜索而非触发 cancel；搜索 MUST 与 include_node 组合（AND）。

  @req:pts5 @human
  场景: left-right-page
    - TreeSelector 翻页 MUST 响应 ←/→（或等价 tui.editor.cursorLeft/cursorRight）以及既有 tui.select.pageUp/pageDown；步长为 max_visible。

  @req:pts6 @human
  场景: status-suffix
    - TreeSelector 状态行 MUST 在 (i/n) 后可选附加 host 提供的 status_suffix（如 filter 标签）；MUST NOT 在包内硬编码产品 FilterMode 枚举。

  @req:pts7 @human
  场景: fold-and-branch-jump
    - TreeSelector MUST 支持折叠可见子树（⊞/⊟ 标记）并通过 tui.tree.foldOrUp / unfoldOrDown（默认 Ctrl/Alt+←→）在 fold 与分支段跳转间切换；搜索或 include_node 变更时 MUST 清空 folds。

  @req:pts8 @human
  场景: node-annotation
    - TreeNode MUST 支持可选 annotation 与 annotation_at；渲染 MUST 在主 label 前显示 [annotation]；show_annotation_timestamps 为真时 MUST 显示 annotation_at。

  @req:pts9 @human
  场景: label-edit-callback
    - TreeSelector MUST 在 tui.tree.editLabel（默认 Shift+L）时调用 on_label_edit(id current_annotation)；MUST 在 tui.tree.toggleLabelTimestamp（默认 Shift+T）时切换时间戳显示。

  @req:pts10 @human
  场景: horizontal-viewport-pan
    - TreeSelector 渲染 MUST 在行宽不足以同时显示 gutter 与选中行 body 时对 body 做水平平移（固定光标 gutter，平移 label/连接符区）；选中行的关键 label 内容在窄宽下 MUST 仍可见；MUST NOT 平移掉选中指示 gutter。

  @req:pts11 @human
  场景: empty-visible-state
    - 当过滤或搜索导致无可见节点时，TreeSelector MUST 渲染可辨识空态行（主题 no_match 或等价）；MUST NOT 静默输出完全空白列表且无提示。

  @req:pts12 @human
  场景: selection-stable-on-filter
    - include_node 或搜索串变更后，若先前选中 id 仍在可见列表中则 MUST 保持该选中；否则 MUST 将选中落到可见列表的合理默认项（通常为首项）。

  @req:pts13 @human
  场景: node-kind
    - TreeNode MUST 支持可选 kind（字符串；包 MUST NOT 硬编码产品 role 枚举）；有 kind 时渲染 MUST 在 annotation 之后、主 label 之前经主题闭包输出 kind 前缀；无 kind 时 MUST NOT 强制前缀。

  @req:pts14 @human
  场景: kind-in-search
    - TreeSelector 增量搜索 MUST 将节点 kind（若有）纳入匹配；搜索 MUST 仍与 include_node 组合（AND）。
