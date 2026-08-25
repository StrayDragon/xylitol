# language: zh-CN
# capability: app-tui-session-tree
# purpose: 产品 TUI 会话树：双 Esc /slash 开 MessageHistory 活树；travel/filter/fold/fork/label 经 Driver（非 c491 假树）。
# scope: 产品 TUI 面

功能: app-tui-session-tree

  @req:ast1 @human
  场景: slot-replace
    - 会话树打开时 MUST 替换 UiRoot editor 槽（贴底可见）；MUST NOT 画成内容区顶部 overlay。

  @req:ast2 @human
  场景: live-travel-via-driver
    - 产品双 Esc 会话树 MUST 渲染当前 session 的 MessageHistory 活树（经 Driver::session_tree(MessageHistory) 映射为包 TreeNode，含 kind）；Enter 选中节点时 MUST 调用 Driver::travel_session_tree(MessageHistory, id)，按返回的 SessionTreeTravel 关闭树、预填 editor（仅当 editor_text 有值）、并重建/刷新 transcript 与后续提交 leaf 语义；MUST NOT 再使用 c491 假树样例作为唯一数据源；filter/fold/fork/label 见本 capability 其余 requirement。

  @req:ast3 @human
  场景: demo-live-session-tree
    - 在产品真 session 图接线前，packages/xylitol-tui agent_demo MUST 维护可变 session_tree：用户提交、工具事件与助手回复结束 MUST 在当前 history leaf 下挂对应节点并推进 leaf；双 Esc 打开的树 MUST 渲染该活树而非仅静态样例。

  @req:ast4 @human
  场景: demo-travel-pi-semantics
    - agent_demo 在会话树 Enter travel 时：若选中节点 kind 为 user，则 MUST 将 history leaf 设为该节点的父（根 user 则回到无叶/约定根策略）、MUST 用该 user 正文预填编辑器（可剥 steer 前缀）、MUST 重建 transcript 为 root→父 路径且 MUST NOT 纳入被选 user 及其后线性回复；若选中非 user，则 MUST 将 leaf 设为选中 id、MUST 重建 root→选中路径、MUST NOT 因 travel 预填 user 正文；完成后 status MUST 回到空闲（无 spinner）；重建后 MUST 以可滚 ScrollNotice（滚动提示）行尾随 history @ 通知（完整 selected/leaf/path 文案），MUST NOT 把该通知插在 transcript 条目最前。

  @req:ast5 @human
  场景: demo-session-tree-fork
    - agent_demo 会话树打开时 MUST 支持 Shift+F fork：重建 root→选中节点路径（MUST NOT 自动纳入其后线性 assistant/tool 回复链）；history leaf MUST 等于选中 id；若选中为 user 消息 MUST 将其文本预填编辑器；关闭树后下一次用户提交 MUST 在该 leaf 下挂新子节点（与既有子树形成兄弟分叉）；MUST NOT 打开新会话文件或调用产品 Driver::fork。

  @req:ast6 @human
  场景: map-session-tree-nodes
    - 产品 MUST 将 domain SessionTreeNode（MessageHistory）映射为 xylitol-tui TreeNode：id 取 entry id、label 为可显示正文（MUST NOT 把 role 烘焙进 label）、kind 取 message role 或等价（user/assistant/tool 等）；SessionTreeNode.label 有值时 MUST 映射为 TreeNode.annotation；bookkeeping 类 entry（ModelChange / ThinkingLevelChange / Label / SessionInfo / Custom / CustomMessage / Header）MUST 映射 kind=meta 以便 default 过滤；主题 kind_prefix 继续由 LayoutTheme 供给。

  @req:ast7 @human
  场景: tree-reflects-persisted-turn
    - 在 auto-persist 与稳定 session_id 就绪后，产品双 Esc 打开的 MessageHistory 树 MUST 能反映已完成回合写入 store 的消息节点；一轮 user/assistant 成功结束后 session_tree(MessageHistory) MUST NOT 因 store 仅有 header 而长期为 0 节点（空会话除外）。

  @req:ast8 @human
  场景: session-tree-filter
    - 产品会话树打开时 MUST 支持与 pi 对齐的 FilterMode（default / no-tools / user-only / labeled-only / all）：经 TreeSelector include_node 应用谓词，并与包增量搜索 AND；default MUST 隐藏 kind=meta；no-tools MUST 在 default 基础上隐藏 kind=tool；user-only MUST 仅 kind=user；labeled-only MUST 仅 annotation 有值；all MUST 显示全部；状态行 MUST 在非 default 时追加 [no-tools]/[user]/[labeled]/[all] 之一，default MUST NOT 追加 [default]；MUST NOT 在 packages/xylitol-tui 硬编码产品 FilterMode。

  @req:ast9 @human
  场景: session-tree-fold
    - 产品会话树打开时 MUST 将 Ctrl+Left / Alt+Left / Ctrl+Right / Alt+Right（包 tui.tree.foldOrUp / unfoldOrDown）转发给 TreeSelector::handle_input，以折叠/展开可折节点或执行分支跳转；折叠后 MUST 隐藏该节点后代并显示 ⊞（或包等价标记）；MUST NOT 在应用面重写折叠算法（复用包 TreeSelector 语义）；裸 Left/Right MUST 仍为翻页（既有）。

  @req:ast10 @human
  场景: product-session-tree-fork
    - 产品会话树 Shift+F MUST 创建新 child session：内容 MUST 为 get_branch 路径并重链 parent_id（对齐 pi createBranchedSession），MUST NOT 按 JSONL 文件序切片，MUST NOT 改写父文件，header MUST 含 parent_session。选中 user 时 MUST 用 ForkPosition::Before（leaf=parent、预填正文、不拷该 user）；选中非 user 时 MUST 用 ForkPosition::At（路径含选中、不因 fork 预填 user 正文）。随后 MUST switch_session 到 child 并关树。MUST NOT 使用 demo 同会话 ast5。

  @req:ast11 @human
  场景: session-tree-slot-help-search
    - 产品会话树槽打开时 MUST 在 TreeSelector 列表上方渲染 Search 行与 TreeHelp 行：Search 在无查询时提示可键入搜索、有查询时显示 Search: 与当前串；TreeHelp MUST 根据 KeybindingsManager（或只读封装）解析当前键位展示 move/page/fold/unfold（折叠）/filters/cycle 等用途片段（可用 · 分隔并随宽度换行），MUST NOT 再使用仅含 Up/Down Enter travel 的过时硬编码唯一提示行作为树槽唯一头行；MUST NOT 把产品 FilterMode 硬编码进 packages/xylitol-tui。

  @req:ast12 @human
  场景: session-tree-label-persist
    - 产品会话树打开时 Shift+L MUST 进入节点 annotation 编辑；Enter 提交后 MUST 经 Driver 将 Label entry 写入当前 session（空串 MUST 清除 annotation）；重开或刷新后 MUST 仍可见对应 TreeNode.annotation；Shift+T MUST 切换 annotation 时间戳显示（本 change 允许本地 just now）；MUST NOT 在产品面 reach infra::session。

  @req:ast13 @human
  场景: debug-scene-tree-fixture
    - 经 /debug session-tree-multiturn 或 /debug session-tree-labeled 装载后，产品双 Esc 打开的 MessageHistory 树 MUST 非空且可见 fixture user 正文；session-tree-labeled MUST 使至少一节点 annotation 有值以便 labeled-only 过滤可验。

  @req:ast14 @human
  场景: session-tree-e2e-pointer
    - 产品会话树 Search/Help 与 Shift+L 行为 MUST 有 tests/tui_e2e 产品 Fake PTY 覆盖（见 test-qa-gate qg05）；本 requirement 为验收指针，不另增运行时行为。

  @req:ast15 @human
  场景: travel-notice-trailing
    - 产品会话树 Enter travel 成功并重建 transcript 后，MUST 以可滚 UiEntry::ScrollNotice（或等价滚动提示行）尾随完整 history @ selected · leaf · path 文案，使跟底时出现在输入框上方视野；重建投影 MUST NOT 将该通知 prepend 为 entries 首条。fork / session 切换 / debug scene 等已有尾随产品 note 的路径 MUST NOT 再叠一条 history @（去重）。

  @req:ast16 @human
  场景: demo-travel-notice-trailing
    - agent_demo 会话树 travel 重建 transcript 后 MUST 与产品同源：history @ 通知 MUST 尾随于路径条目之后；MUST NOT 在 clear 后先插 ScrollNotice 再推路径（旧顶插）。
