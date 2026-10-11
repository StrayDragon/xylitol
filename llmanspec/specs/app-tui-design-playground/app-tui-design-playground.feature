# language: zh-CN
# capability: app-tui-design-playground
# purpose: 交互设计稿：token 从视觉 SSOT 生成；模块固定态可闸；无快捷键墙；浏览器稿不是运行时真值。
# scope: designing/, src/app/tui/
# reviewed-through: c2890

功能: app-tui-design-playground

  @req:r1206
  规则: token-sync-ssot
    产品 TUI 语义色 须以视觉 SSOT frontmatter 为唯一色板数据；designing 生成的 tokens 须由单一同步脚本写出；手改生成物 不得作为长期真值。
    # verified-by: designing/AGENTS.md
    场景: token-sync-single-script
      当 读取 token 同步脚本与生成物
      那么 单一脚本写出双端 token

  @req:r1207
  规则: markdown-slot-align
    Markdown 固定态示意 须无井号标题前缀、链接为 text (url)、粗体斜体无可见星号包裹。
    # verified-by: designing/AGENTS.md
    场景: markdown-body-styled-no-literal-markers
      当 以场景构建器渲染多段助手正文（宽 80）
      那么 助手正文行携带样式转义

  @req:r1208
  规则: agent-default-ignore
    面 AGENTS 须写明 Agent 改表面先读产品代码、默认读短模块 intent/states、默认忽略 designing 应用壳；仅在人类点名路径时才读应用壳源码。
    # verified-by: designing/AGENTS.md
    场景: tui-agents-reading-order
      当 读取 tui 面 AGENTS 摘要
      那么 摘要写明先读产品代码与默认忽略应用壳

  @req:r1209
  规则: session-tree-kind-ssot
    会话树固定态 须用 DESIGN token 色区分 kind 前缀；选中反转行 须继承前景，不得再把 role 字符串预烘焙成唯一表现。
    # verified-by: llmanspec/specs/app-tui-input/app-tui-input.feature
    场景: kind-prefix-themed-not-baked-role
      当 以带 kind 的样例树挂载
      那么 kind 前缀经主题闭包渲染在主标签前

  @req:r1210
  规则: session-tree-travel-ssot-live
    会话树是 Driver MessageHistory 活树；Enter 经 travel（或等价）预填 user 输入。不得再暗示产品仍为假树 stub。
    # verified-by: llmanspec/specs/app-tui-input/app-tui-input.feature
    场景: live-tree-from-command
      当 以主机泵在 idle 提交斜杠 session-tree
      那么 会话树打开且未作为 prompt 调用 run

  @req:r1213
  规则: static-slots
    交互设计稿 须提供 models（wide / narrow / no-thinking）、session-tree filter 与 pending next-turn 固定态；不得在静图中表达实现分层（包标签）或顶栏快捷键墙；不得设立独立快捷键百科模块。
    # verified-by: designing/AGENTS.md
    场景: static-slot-samples-covered
      当 枚举 designing 固定态样例
      那么 固定态样例覆盖模型与树与待办

  @req:r1211
  规则: designing-lint-gate
    仓库 须提供 designing lint 脚本（或等价），并由 just qa 经 check-scripts 执行；失败时 须非零退出。脚本 须覆盖：模块 intent/draft/states 存在、states 的 must_contain / must_not_contain、禁止实现分层标签、选中反转 inherit、无独立 keybindings 模块。
    # verified-by: scripts/check_tui_designing.py
    场景: designing-lint-in-qa
      当 读取 designing lint 闸接线
      那么 designing lint 由 check-scripts 执行

  @req:r1212
  规则: designing-lint-docs
    面 AGENTS 须写明改设计稿须跑 designing lint 与 check-tui-tokens；不得再暗示仅靠人眼发现色板或固定态漂移。
    场景: tui-agents-name-lint-commands
      当 读取 tui 面 AGENTS 摘要
      那么 摘要写明改稿须跑 designing lint

    # verified-by: designing/AGENTS.md
