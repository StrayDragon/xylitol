# language: zh-CN
# managed by llman sdd partition-migrate
功能: app-tui-design-playground

  @req:adp1
  场景: sync-runs
    当 运行 token 同步脚本
    那么 生成或刷新 token 生成物且无报错

  @req:adp2
  场景: md-slot
    当 打开 Markdown 固定态
    那么 示意中无井号标题前缀且链接呈 text (url) 形态

  @req:adp2
  场景: no-star-wrapping
    当 查看粗体斜体示意
    那么 可见文案无 ** 或成对 * 包裹

  @req:adp3
  场景: agents-pointer
    当 阅读面 AGENTS 设计稿约定
    那么 写明 Agent 先读产品代码、默认读短模块、默认忽略 designing 应用壳

  @req:adp4
  场景: tree-slot-kind-colors
    假如 打开会话树固定态
    当 查看树行
    那么 可见分色 kind 前缀与选中反转 inherit

  @req:adp5
  场景: live-tree-note
    当 阅读会话树设计备注
    那么 写明活树与 travel（或等价）/ user 预填

  @req:adp9
  场景: slots-present
    当 打开 designing
    那么 存在 models、session-tree、pending 模块且无包标签、无顶栏快捷键墙、无独立 keybindings 模块

  @req:adp9
  场景: models-shape
    当 查看 models 打开态
    那么 替换 editor 槽示意且选中行为整行 reverse

  @req:adp9
  场景: models-wide-levels
    当 查看 models.wide 固定态
    那么 焦点行铺开多档 xylitol level 且当前档有方括号标记

  @req:adp9
  场景: pending-trail
    当 查看 pending next-turn 固定态
    那么 status 含 Next turn 文案且贴在 busy 行右侧

  @req:adp6
  场景: qa-runs-lint
    当 运行 designing lint --check
    那么 退出码 0 或对故意违规样本非 0

  @req:adp6
  场景: rev-inherit
    假如 设计稿 CSS 含选中反转
    当 检查选中对比度规则
    那么 要求 color inherit 且选中行无 kind 前景穿透

  @req:adp8
  场景: docs-pointer
    当 阅读面 AGENTS 设计稿约定
    那么 含 lint / qa 指针
