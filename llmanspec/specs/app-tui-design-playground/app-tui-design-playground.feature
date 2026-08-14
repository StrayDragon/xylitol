# language: zh-CN
# managed by llman sdd partition-migrate
功能: app-tui-design-playground

  @req:adp1
  场景: sync-runs
    当 运行 token 同步脚本
    那么 生成或刷新 token 生成物且无报错

  @req:adp6
  场景: qa-runs-lint
    当 运行 playground lint --check
    那么 退出码 0 或对故意违规样本非 0

  @req:adp6
  场景: rev-inherit
    假如 playground CSS 含 .rev
    当 检查选中对比度规则
    那么 要求 .rev * color inherit 且 TREEP 选中行无 fg-user 穿透
