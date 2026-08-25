# language: zh-CN
# capability: app-tui-trust
# purpose: 产品 TUI 项目信任选择器（ChoicePrompt）；信任后 yolo。
# scope: 产品 TUI 面

功能: app-tui-trust

  @req:atr1 @human
  场景: choice-prompt-not-stdio
    - 产品 TUI 路径（默认裸跑或 tui 动词）在需要 Ask trust 时 MUST 在 raw-mode TUI 内用 packages/xylitol-tui ChoicePrompt（替换 editor 槽）完成选择；MUST NOT 调用 prompt_trust_options_stdio 或等价 stderr 数字菜单。

  @req:atr2 @human
  场景: theme-and-cancel
    - Trust ChoicePrompt MUST 使用 Palette::dark()（或等价）choice_prompt_theme；Esc/取消 MUST 视为不信任（deny）；提交 MUST 经 TrustManager 写入持久 store。

  @req:atr3 @human
  场景: yolo-after
    - 项目已信任后产品 TUI MUST NOT 实现逐工具审批 UI；工具默认执行；MUST 保留 hook 扩展点。

  @req:atr4 @human
  场景: slash-persist-no-auto-reload
    - 产品 /trust slash MUST 经 Driver/composition 缝调用 TrustManager（或等价端口）持久化 cwd/parent/deny 决策；MUST NOT 从 app/tui 直接 reach infra::trust；MUST NOT 解冻 Choice/Plate 活板作为本命令唯一路径；写盘成功后本会话 MUST NOT 自动重载项目 skills/MCP/context（用户显式 /reload 或重启除外）。
