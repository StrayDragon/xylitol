# language: zh-CN
# managed by llman sdd partition-migrate
功能: app-tui-trust

  @req:r27
  场景: shell-present
    当 列出 llmanspec/specs/app-tui-trust
    那么 spec.toon 存在且 purpose 为中文

  @req:atr1
  场景: no-stdio-menu
    假如 需要 Ask 且启动 --tui
    当 进入产品 TUI
    那么 editor 槽为 ChoicePrompt 且无 Choice [1-N] stderr 菜单

  @req:atr1
  场景: options-from-store
    当 构造 trust 选择
    那么 选项 label 来自 TrustManager::get_trust_options

  @req:atr2
  场景: esc-denies
    假如 ChoicePrompt 已打开
    当 按 Esc
    那么 不信任且不写 trust 肯定决策（或等价 deny）

  @req:atr3
  场景: no-tool-approval-ui
    假如 项目已信任
    当 模型请求工具
    那么 无逐工具确认弹层

  @req:atr4
  场景: persist-cwd
    假如 idle 且 trust store 可测
    当 /trust
    那么 store 含当前 cwd 信任决策

  @req:atr4
  场景: no-auto-reload
    假如 idle
    当 /trust 成功
    那么 reload_runtime 调用次数为 0
