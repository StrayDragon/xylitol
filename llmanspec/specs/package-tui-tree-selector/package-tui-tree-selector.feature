# language: zh-CN
# capability: package-tui-tree-selector
# purpose: 通用 TreeSelector：flatten/过滤/搜索/翻页/fold/annotation（供双 Esc 会话树）。
# scope: packages/xylitol-tui/

功能: package-tui-tree-selector

  @req:r1677 @human
  场景: tree-node-model
    - 包 MUST 提供 TreeNode（id/label/children）与可导航 TreeSelector 组件；MUST NOT 依赖主 crate xylitol 或 session 类型。

  @req:r1683 @human
  场景: include-node-filter
    - TreeSelector MUST 支持 include_node 钩子过滤可见节点，并在过滤后重算 indent/connector/gutter；MUST NOT 硬编码产品 FilterMode 枚举。

  @req:r1684 @human
  场景: select-keybindings
    - TreeSelector 导航 MUST 使用 tui.select.* 键绑定（up/down/page/confirm/cancel）；选中态 MUST 可用主题闭包（如 reverse）表达。

  @req:r1685 @human
  场景: incremental-search
    - TreeSelector MUST 支持对可见节点 label 的增量搜索（可打印字符追加、Backspace 删除）；有搜索串时 Esc MUST 先清空搜索而非触发 cancel；搜索 MUST 与 include_node 组合（AND）。

  @req:r1686 @human
  场景: left-right-page
    - TreeSelector 翻页 MUST 响应 ←/→（或等价 tui.editor.cursorLeft/cursorRight）以及既有 tui.select.pageUp/pageDown；步长为 max_visible。

  @req:r1687 @human
  场景: status-suffix
    - TreeSelector 状态行 MUST 在 (i/n) 后可选附加 host 提供的 status_suffix（如 filter 标签）；MUST NOT 在包内硬编码产品 FilterMode 枚举。

  @req:r1688 @human
  场景: fold-and-branch-jump
    - TreeSelector MUST 支持折叠可见子树（⊞/⊟ 标记）并通过 tui.tree.foldOrUp / unfoldOrDown（默认 Ctrl/Alt+←→）在 fold 与分支段跳转间切换；搜索或 include_node 变更时 MUST 清空 folds。

  @req:r1689 @human
  场景: node-annotation
    - TreeNode MUST 支持可选 annotation 与 annotation_at；渲染 MUST 在主 label 前显示 [annotation]；show_annotation_timestamps 为真时 MUST 显示 annotation_at。

  @req:r1690 @human
  场景: label-edit-callback
    - TreeSelector MUST 在 tui.tree.editLabel（默认 Shift+L）时调用 on_label_edit(id current_annotation)；MUST 在 tui.tree.toggleLabelTimestamp（默认 Shift+T）时切换时间戳显示。

  @req:r1678 @human
  场景: horizontal-viewport-pan
    - TreeSelector 渲染 MUST 在行宽不足以同时显示 gutter 与选中行 body 时对 body 做水平平移（固定光标 gutter，平移 label/连接符区）；选中行的关键 label 内容在窄宽下 MUST 仍可见；MUST NOT 平移掉选中指示 gutter。

  @req:r1679 @human
  场景: empty-visible-state
    - 当过滤或搜索导致无可见节点时，TreeSelector MUST 渲染可辨识空态行（主题 no_match 或等价）；MUST NOT 静默输出完全空白列表且无提示。

  @req:r1680 @human
  场景: selection-stable-on-filter
    - include_node 或搜索串变更后，若先前选中 id 仍在可见列表中则 MUST 保持该选中；否则 MUST 将选中落到可见列表的合理默认项（通常为首项）。

  @req:r1681 @human
  场景: node-kind
    - TreeNode MUST 支持可选 kind（字符串；包 MUST NOT 硬编码产品 role 枚举）；有 kind 时渲染 MUST 在 annotation 之后、主 label 之前经主题闭包输出 kind 前缀；无 kind 时 MUST NOT 强制前缀。

  @req:r1682 @human
  场景: kind-in-search
    - TreeSelector 增量搜索 MUST 将节点 kind（若有）纳入匹配；搜索 MUST 仍与 include_node 组合（AND）。

  @req:r1677 @executable
  场景: tree-selector-navigable-model-headless
    当 以样例树挂载 TreeSelector
    那么 根与子节点行可见且选中态落在首项

  @req:r1683 @executable
  场景: include-node-filter-rebuilds-headless
    当 以样例树挂载 TreeSelector 并设置只含叶子的过滤
    那么 可见列表仅剩叶子且行完整重算
    当 设置放行全部的过滤
    那么 树行完整恢复

  @req:r1684 @executable
  场景: select-keybindings-and-theme-cursor-headless
    当 以样例树挂载 TreeSelector
    当 按下 select down 再按下 select up
    那么 选中态回到首项
    当 按下 select confirm
    那么 on_select 回调收到该节点 id
    当 按下 select cancel
    那么 on_cancel 回调触发
    并且 选中行经主题闭包反色渲染

  @req:r1685 @executable
  场景: incremental-search-esc-clears-first-headless
    当 以样例树挂载 TreeSelector 并输入搜索串 "child"
    那么 可见列表收窄到匹配项
    当 退格删除一个字符
    那么 搜索串变为 "chil"
    当 按下 select cancel 且搜索串非空
    那么 搜索被清空且取消回调未触发
    当 设置只含叶子的过滤并搜索 "second"
    那么 搜索与过滤按 AND 组合

  @req:r1686 @executable
  场景: page-keys-move-by-max-visible-headless
    当 以 30 叶树挂载并作用域绑定翻页键
    当 按下 select pageDown
    那么 选中前进 max_visible
    当 按下 select pageUp
    那么 选中回到首项
    当 默认键表下按左右方向键
    那么 选中保持不变

  @req:r1687 @executable
  场景: status-suffix-after-counter-headless
    当 以样例树挂载并设置状态后缀 "[no-tools]"
    那么 状态行在 (i/n) 后附加该后缀

  @req:r1688 @executable
  场景: fold-and-branch-jump-keys-headless
    当 以样例树挂载并把选中移到可折叠父节点
    当 按下 tree foldOrUp
    那么 该节点子树折叠且渲染出现折叠标记
    当 按下 tree unfoldOrDown
    那么 子树重新展开
    当 输入搜索字符
    那么 折叠态被清空

  @req:r1689 @executable
  场景: node-annotation-renders-headless
    当 以带注解与时间戳的样例树挂载
    那么 注解以括号形式先于主标签渲染
    当 开启时间戳显示
    那么 注解时间戳随之出现

  @req:r1690 @executable
  场景: label-edit-and-timestamp-toggle-keys-headless
    当 以带注解的样例树挂载并选中该节点
    当 按下 tree editLabel
    那么 on_label_edit 收到 id 与当前注解
    当 按下 tree toggleLabelTimestamp
    那么 时间戳显示翻转

  @req:r1678 @executable
  场景: horizontal-viewport-pan-narrow-width-headless
    当 以超长标签树挂载并按窄宽渲染
    那么 选中行保留光标 gutter 且标签尾部内容仍可见

  @req:r1679 @executable
  场景: empty-visible-state-row-headless
    当 以样例树挂载并过滤到空集
    那么 渲染出现可辨识空态行且计数为 (0/0)

  @req:r1680 @executable
  场景: selection-stable-on-filter-change-headless
    当 以样例树挂载并选中仍可见的节点后变更过滤
    那么 选中保持该节点
    当 变更为排除该节点的过滤
    那么 选中落到可见首项

  @req:r1681 @executable
  场景: node-kind-prefix-headless
    当 以带 kind 的样例树挂载
    那么 kind 前缀经主题闭包渲染在主标签前
    当 以无 kind 的样例树挂载
    那么 渲染不强制 kind 前缀

  @req:r1682 @executable
  场景: kind-matches-in-search-headless
    当 以带 kind 的样例树挂载并搜索 kind 词
    那么 该节点经 kind 匹配保持可见
