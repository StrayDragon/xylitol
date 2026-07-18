# language: zh-CN
# managed by llman sdd partition-migrate
功能: app-tui-session-tree

  @req:ast1
  场景: double-esc-opens
    假如 空编辑器
    当 双 Esc 于 500ms 内
    那么 editor 槽显示 Session tree

  @req:ast2
  场景: open-shows-live
    假如 Driver 当前 session 已有 user/assistant 消息
    当 空编辑器双 Esc 打开树
    那么 树行来自 MessageHistory 活数据而非 sample_session_tree 假树

  @req:ast3
  场景: submit-grows-tree
    假如 agent_demo 已打开
    当 提交唯一 user 文本
    那么 session_tree 含该文本且再开树可见

  @req:ast3
  场景: tool-grows-tree
    假如 agent_demo 脚本轮产生 Tool
    当 推进 tick 至工具出现
    那么 session_tree 含 tool: 节点

  @req:ast4
  场景: travel-user-prefills-editor
    假如 样例树选中 u1（kind=user，其下有 assistant 子节点）
    当 Enter travel
    那么 编辑器含原 user 正文、leaf 为 u1 的父、transcript 不含 u1 子 assistant 正文、树关闭且 status 空闲

  @req:ast4
  场景: travel-user-omits-reply-spine
    假如 同上
    当 Enter travel
    那么 transcript 不含 travel 前跟在 u1 后的线性 assistant/tool 回复

  @req:ast4
  场景: travel-assistant-no-prefill
    假如 选中 assistant 节点
    当 Enter travel
    那么 leaf 为该 assistant id 且编辑器未因 travel 填入 user 正文

  @req:ast5
  场景: fork-stays-on-user
    假如 树选中样例 u1（其下已有 assistant 子节点）
    当 Shift+F 或 fork_from_history
    那么 leaf 为 u1、编辑器含原 user 文案、transcript 不含子 assistant 正文

  @req:ast5
  场景: submit-creates-sibling
    假如 fork 到 u1 之后
    当 提交新 user 文本
    那么 u1 的 children 数量加一且活树含新文案

  @req:ast6
  场景: annotation-mapped
    假如 活树节点域 label 已 resolve
    当 打开会话树
    那么 对应 TreeNode.annotation 有值

  @req:ast7
  场景: after-turn-tree-nonempty
    假如 产品 TUI 完成至少一轮对话且消息已 persist
    当 双 Esc 打开会话树
    那么 树节点数大于 0 且可见 user 正文

  @req:ast8
  场景: ctrl-t-hides-tools
    假如 产品树已打开且含 tool 行
    当 按 Ctrl+T 进入 no-tools
    那么 可见行无 kind=tool 且状态含 [no-tools]

  @req:ast8
  场景: toggle-back-default
    假如 树在 no-tools
    当 再按 Ctrl+T
    那么 回到 default 谓词且状态无 [no-tools]

  @req:ast8
  场景: labeled-only
    假如 树含带 annotation 与无 annotation 节点
    当 Ctrl+L 进入 labeled-only
    那么 仅 annotation 有值的行可见

  @req:ast9
  场景: fold-hides-descendants
    假如 树开且选中有子节点的可折行
    当 按 Ctrl+Left（或 Alt+Left）
    那么 该节点 is_folded 且后代行不可见，帧含 ⊞ 或等价

  @req:ast9
  场景: unfold-restores
    假如 同上已折叠
    当 按 Ctrl+Right（或 Alt+Right）
    那么 后代重新可见

  @req:ast10
  场景: fork-new-session
    假如 树开且选中节点
    当 按 Shift+F
    那么 fork+switch 后当前为 child、树关闭、父 JSONL 未改

  @req:ast10
  场景: fork-user-before
    假如 树选中 kind=user
    当 Shift+F
    那么 Before：editor 含正文且 child 不含该 user id

  @req:ast10
  场景: fork-non-user-at
    假如 树选中 assistant/tool
    当 Shift+F
    那么 At：child 含该节点 id 且 editor 未因 fork 填 user 正文

  @req:ast10
  场景: fork-no-sibling-leak
    假如 父会话同层已有兄弟分支
    当 对一侧路径 Shift+F
    那么 child 不含另一侧兄弟条目

  @req:ast11
  场景: open-shows-search-help
    假如 产品 TUI idle 且已打开会话树
    当 渲染 Tree 槽一帧
    那么 帧内含 Search 或 Type to search 类提示，且含由键位解析得到的 move 或 filters 或 cycle 等 Help 片段

  @req:ast11
  场景: search-echo
    假如 产品会话树已打开
    当 键入搜索串 foo
    那么 Search 行显示含 foo 的查询回显

  @req:ast12
  场景: label-persist
    假如 产品树已打开且选中节点
    当 Shift+L 输入 keep 后 Enter
    那么 树行含 [keep] 且 store 有对应该节点的 Label entry

  @req:ast13
  场景: after-debug-tree-nonempty
    假如 已成功 /debug session-tree-multiturn
    当 双 Esc 开树
    那么 树节点数大于 0 且可见 fixture user 正文

  @req:ast14
  场景: e2e-case-exists
    当 列出 tests/tui_e2e/pty.rs
    那么 存在产品会话树 #[ignore] 用例且注释或名含 session_tree
