# language: zh-CN
# capability: app-tui-trust
# purpose: 产品 TUI 项目信任选择器（ChoicePrompt）；信任后 yolo。
# scope: src/app/tui/

功能: app-tui-trust

  @req:r1374
  规则: choice-prompt-not-stdio
    产品 TUI 路径（默认裸跑或 tui 动词）在需要 Ask trust 时 MUST 在 raw-mode TUI 内用 packages/xylitol-tui ChoicePrompt（替换 editor 槽）完成选择；MUST NOT 调用 prompt_trust_options_stdio 或等价 stderr 数字菜单。
    # verified-by: llmanspec/specs/app-tui-ask/app-tui-ask.feature
  @req:r1375
  规则: theme-and-cancel
    Trust ChoicePrompt MUST 使用 Palette::dark()（或等价）choice_prompt_theme；Esc/取消 MUST 视为不信任（deny）；提交 MUST 经 TrustManager 写入持久 store。
    # verified-by: llmanspec/specs/app-tui-trust/app-tui-trust.feature
  @req:r1376
  规则: yolo-after
    项目已信任后产品 TUI MUST NOT 实现逐工具审批 UI；工具默认执行；MUST 保留 hook 扩展点。
    # verified-by: llmanspec/specs/domain-security/domain-security.feature
  @req:r1377
  规则: slash-persist-no-auto-reload
    产品 /trust slash MUST 经 Driver/composition 缝调用 TrustManager（或等价端口）持久化 cwd/parent/deny 决策；MUST NOT 从 app/tui 直接 reach infra::trust；MUST NOT 解冻 Choice/Plate 活板作为本命令唯一路径；写盘成功后本会话 MUST NOT 自动重载项目 skills/MCP/context（用户显式 /reload 或重启除外）。
# re-review(c2826): 复审结论——本 capability 管辖行为不变；分支内改动仅测试基建与可见性再导出（2026-09-28）

# re-review(c2827): 复审结论——本 capability 管辖行为不变；分支内改动为 BDD 场景落地、BDD 测试基建（steps/bindings/驱动旋钮与探针）与可见性再导出（2026-09-28）
    # verified-by: llmanspec/specs/app-tui-trust/app-tui-trust.feature
