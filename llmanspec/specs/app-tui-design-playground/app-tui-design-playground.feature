# language: zh-CN
# managed by llman sdd partition-migrate
功能: app-tui-design-playground

  @req:adp0
  场景: shell-present
    当 列出 capability app-tui-design-playground
    那么 spec 含 adp0 及后续 requirements

  @req:adp1
  场景: sync-runs
    当 运行 sync_tokens.py
    那么 生成或刷新 tokens.css 与 tokens.js 且无报错

  @req:adp2
  场景: md-slot
    当 打开 playground Markdown 槽
    那么 示意中无井号标题前缀且链接呈 text (url) 形态

  @req:adp2
  场景: no-star-wrapping
    当 查看粗体斜体示意
    那么 可见文案无 ** 或成对 * 包裹

  @req:adp3
  场景: agents-pointer
    当 阅读 design/AGENTS.md
    那么 含 Agent 默认忽略 playground 的说明

  @req:adp4
  场景: tree-slot-kind-colors
    假如 打开 playground 会话树槽
    当 查看树行
    那么 可见分色 kind 前缀与无 role 烘焙的正文

  @req:adp5
  场景: playground-live-note
    假如 打开 design playground 会话树槽
    当 阅读注脚
    那么 写明活树与 travel_session_tree / user 预填

  @req:adp9
  场景: slots-present
    当 打开 design playground
    那么 存在 Models 与 Tree power 槽且无包标签置灰

  @req:adp9
  场景: models-shape
    当 查看 Models 打开态
    那么 替换 editor 槽示意且选中行为整行 reverse

  @req:adp10
  场景: design-table
    当 阅读 DESIGN.md Next wave 表
    那么 列出后续 change 与文档指针

  @req:adp6
  场景: qa-runs-lint
    当 运行 python3 scripts/check_tui_design_playground.py --check
    那么 退出码 0 或对故意违规样本非 0

  @req:adp6
  场景: rev-inherit
    假如 playground CSS 含 .rev
    当 检查选中对比度规则
    那么 要求 .rev * color inherit 且 TREEP 选中行无 fg-user 穿透

  @req:adp7
  场景: fixtures-present
    当 列出 design/fixtures
    那么 存在 session-tree.filter 与 models.open YAML 且 HTML 有对应 data-design-fixture

  @req:adp8
  场景: docs-pointer
    当 阅读 design/AGENTS.md 与 playground README
    那么 含 lint / qa 指针
