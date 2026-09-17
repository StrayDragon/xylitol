# language: zh-CN
# capability: app-tui-design-playground
# purpose: 交互设计稿：token 从视觉 SSOT 生成；模块固定态可闸；无快捷键墙；浏览器稿不是运行时真值。
# scope: designing/, src/app/tui/

功能: app-tui-design-playground

  @req:r1206 @human
  场景: token-sync-ssot
    - 产品 TUI 语义色 MUST 以视觉 SSOT frontmatter 为唯一色板数据；designing 生成的 tokens MUST 由单一同步脚本写出；手改生成物 MUST NOT 作为长期真值。

  @req:r1207 @human
  场景: markdown-slot-align
    - Markdown 固定态示意 MUST 无井号标题前缀、链接为 text (url)、粗体斜体无可见星号包裹。

  @req:r1208 @human
  场景: agent-default-ignore
    - 面 AGENTS MUST 写明 Agent 改表面先读产品代码、默认读短模块 intent/states、默认忽略 designing 应用壳；仅在人类点名路径时才读应用壳源码。

  @req:r1209 @human
  场景: session-tree-kind-ssot
    - 会话树固定态 MUST 用 DESIGN token 色区分 kind 前缀；选中反转行 MUST 继承前景，MUST NOT 再把 role 字符串预烘焙成唯一表现。

  @req:r1210 @human
  场景: session-tree-travel-ssot-live
    - 会话树是 Driver MessageHistory 活树；Enter 经 travel（或等价）预填 user 输入。MUST NOT 再暗示产品仍为假树 stub。

  @req:r1213 @human
  场景: static-slots
    - 交互设计稿 MUST 提供 models（wide / narrow / no-thinking）、session-tree filter 与 pending next-turn 固定态；MUST NOT 在静图中表达实现分层（包标签）或顶栏快捷键墙；MUST NOT 设立独立快捷键百科模块。

  @req:r1211 @human
  场景: designing-lint-gate
    - 仓库 MUST 提供 designing lint 脚本（或等价），并由 just qa 经 check-scripts 执行；失败时 MUST 非零退出。脚本 MUST 覆盖：模块 intent/draft/states 存在、states 的 must_contain / must_not_contain、禁止实现分层标签、选中反转 inherit、无独立 keybindings 模块。

  @req:r1212 @human
  场景: designing-lint-docs
    - 面 AGENTS MUST 写明改设计稿须跑 designing lint 与 check-tui-tokens；MUST NOT 再暗示仅靠人眼发现色板或固定态漂移。
