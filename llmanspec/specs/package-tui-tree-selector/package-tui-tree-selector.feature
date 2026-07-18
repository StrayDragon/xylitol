# language: zh-CN
# managed by llman sdd partition-migrate
功能: package-tui-tree-selector

  @req:pts1
  场景: unit-flatten
    当 对分支树 flatten
    那么 单子链 indent 不漂移且分支点出现连接符

  @req:pts2
  场景: filter-recalc
    当 隐藏中间节点
    那么 可见兄弟 indent 对齐且无幽灵连接符

  @req:pts3
  场景: nav-confirm
    当 ↓ 后 Enter
    那么 on_select 收到对应节点 id

  @req:pts4
  场景: search-and-esc
    假如 树有多节点
    当 键入匹配子串后 Esc
    那么 仅匹配节点可见且搜索清空后恢复；第二次 Esc 才 cancel

  @req:pts5
  场景: arrow-page
    假如 可见行多于一页
    当 按 → 或 pageDown
    那么 selected_index 增加约 max_visible 且不越界

  @req:pts6
  场景: suffix-render
    假如 host 设置 status_suffix=[no-tools]
    当 render
    那么 状态行含 (i/n) 与 [no-tools]

  @req:pts7
  场景: fold-hides-desc
    假如 可折叠节点选中
    当 foldOrUp
    那么 子节点从可见列表消失且前缀含 ⊞

  @req:pts8
  场景: annotation-render
    假如 节点带 annotation
    当 render
    那么 行含 [annotation] 与主 label

  @req:pts9
  场景: edit-label-fires
    假如 选中节点
    当 Shift+L
    那么 on_label_edit 收到该 id

  @req:pts10
  场景: deep-indent-visible
    假如 深嵌套树且 active/selected 为长 label 叶节点
    当 以窄 width render
    那么 输出含叶 label 的可辨识子串且行仍以选中 gutter 开头

  @req:pts11
  场景: no-match
    假如 树有节点
    当 搜索无匹配串
    那么 render 含空态提示文案

  @req:pts12
  场景: keep-id
    假如 选中某仍可见节点
    当 收紧 include_node 但仍含该 id
    那么 选中仍为该 id

  @req:pts12
  场景: fallback
    假如 选中节点被过滤掉
    当 刷新可见列表
    那么 选中变为可见首项或等价默认

  @req:pts13
  场景: kind-prefix-render
    假如 节点 kind=user 且 label 为正文无 role 前缀
    当 render
    那么 行含主题化 user 前缀与正文 label

  @req:pts13
  场景: no-kind-compat
    假如 节点无 kind
    当 render
    那么 行仅含既有 annotation/label 形态且无强制 role 前缀

  @req:pts14
  场景: search-matches-kind
    假如 可见节点含 kind=tool
    当 键入 tool
    那么 该节点仍可见
