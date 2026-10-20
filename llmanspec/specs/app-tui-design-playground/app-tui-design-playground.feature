# language: zh-CN
# purpose: 产品视觉色板 SSOT：DESIGN.md frontmatter ↔ Palette；无平行设计稿目录。

功能: app-tui-design-playground

  @req:r1206
  规则: token-sync-ssot
    产品 TUI 语义色 须以视觉 SSOT frontmatter 为唯一色板数据；Palette 须由单一检查脚本对照 DESIGN.md；手改 Palette hex 与 DESIGN 不一致时 须非零退出。
    场景: 词元同步单一脚本
      当 读取 token 检查脚本与色板
      那么 单一脚本对照 DESIGN 与 Palette

  @req:r1207
  规则: markdown-slot-align
    Markdown 固定态示意 须无井号标题前缀、链接为 text (url)、粗体斜体无可见星号包裹。
    场景: Markdown正文已样式化无字面标记
      当 以场景构建器渲染多段助手正文（宽 80）
      那么 助手正文行携带样式转义

  @req:r1208
  规则: agent-reading-order
    面 AGENTS 须写明 Agent 改表面先读本目录产品代码与 DESIGN.md；包 demo 不得充当产品视觉 SSOT。
    场景: TUIagent阅读顺序
      当 读取 tui 面 AGENTS 摘要
      那么 摘要写明先读产品代码与 DESIGN.md

  @req:r1209
  规则: session-tree-kind-ssot
    会话树固定态 须用 DESIGN token 色区分 kind 前缀；选中反转行 须继承前景，不得再把 role 字符串预烘焙成唯一表现。
    场景: 种类前缀主题化不写死角色
      当 以带 kind 的样例树挂载
      那么 kind 前缀经主题闭包渲染在主标签前

  @req:r1210
  规则: session-tree-travel-ssot-live
    会话树是 Driver MessageHistory 活树；Enter 经 travel（或等价）预填 user 输入。不得再暗示产品仍为假树 stub。
    场景: 在线树来自命令
      当 以主机泵在 idle 提交斜杠 session-tree
      那么 会话树打开且未作为 prompt 调用 run

  @req:r1213
  规则: static-slots
    产品视觉 SSOT 须覆盖 models picker、会话树筛选与待办栏语义；不得在产品面表达实现分层标签或顶栏快捷键墙；不得设立独立快捷键百科模块。
    场景: 静态槽样例覆盖
      当 枚举产品视觉槽位
      那么 视觉槽位覆盖模型与树与待办

  @req:r1211
  规则: token-palette-gate
    仓库 须提供 DESIGN.md 与 Palette 一致性检查，并由 just qa 经 qa-check-tui-tokens 执行；失败时 须非零退出。
    场景: 词元校验于门禁
      当 读取 token 闸接线
      那么 token 闸由 qa-check-tui-tokens 执行

  @req:r1212
  规则: token-check-docs
    面 AGENTS 须写明改色板须跑 qa-check-tui-tokens；不得再暗示仅靠人眼发现色板漂移。
    场景: TUIagent名称检查命令
      当 读取 tui 面 AGENTS 摘要
      那么 摘要写明改色板须跑 token 闸
