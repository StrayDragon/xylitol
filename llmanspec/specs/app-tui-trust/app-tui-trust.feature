# language: zh-CN
# purpose: 产品 TUI 项目信任选择器（ChoicePrompt）；信任后 yolo。

功能: app-tui-trust

  @req:r1374
  规则: choice-prompt-not-stdio
    产品 TUI 路径（默认裸跑或 tui 动词）在需要 Ask trust 时 须在 raw-mode TUI 内用 crates/xylitol-tui ChoicePrompt（替换 editor 槽）完成选择；不得调用 prompt_trust_options_stdio 或等价 stderr 数字菜单。

    场景: 信任问卷使用选择提示词于TUI
      假如 项目尚未信任
      当 启动需 Trust 闸的路径
      那么 仍走 Trust ChoicePrompt 而非 ask 工具

    场景: 选择槽替换编辑器
      假如 ChoicePrompt 因 ask 打开
      当 产品 TUI 渲染 editor 槽
      那么 槽为 ChoicePrompt 且标题含 Ask
  @req:r1375
  规则: theme-and-cancel
    Trust ChoicePrompt 须使用 Palette::dark()（或等价）choice_prompt_theme；Esc/取消 须视为不信任（deny）；提交 须经 TrustManager 写入持久 store。

    场景: 信任选择主题与取消
      当 读取 trust 选择器主题与取消收口
      那么 主题出自 dark 且取消记为不信任
  @req:r1376
  规则: yolo-after
    项目已信任后产品 TUI 不得实现逐工具审批 UI；工具默认执行；须保留 hook 扩展点。

    场景: 钩子点保持之后信任
      假如 注册匹配 bash 的 before 拒绝 hook
      当 运行 AgentRuntime 触发 bash
      那么 tool-error 回写且未执行
  @req:r1377
  规则: slash-persist-no-auto-reload
    产品 /trust slash 须经 Driver/composition 缝调用 TrustManager（或等价端口）持久化 cwd/parent/deny 决策；不得从 app/tui 直接 reach infra::trust；不得解冻 Choice/Plate 活板作为本命令唯一路径；写盘成功后本会话 不得自动重载项目 skills/MCP/context（用户显式 /reload 或重启除外）。

    场景: 信任斜杠持久化无自动重载
      当 读取 trust slash 的缝接线
      那么 经 Driver 缝持久化且本会话不自动重载
