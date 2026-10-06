# language: zh-CN
# capability: app-tui-trust
# purpose: 产品 TUI 项目信任选择器（ChoicePrompt）；信任后 yolo。
# scope: src/app/tui/

功能: app-tui-trust

  @req:r1374
  规则: choice-prompt-not-stdio
    产品 TUI 路径（默认裸跑或 tui 动词）在需要 Ask trust 时 MUST 在 raw-mode TUI 内用 packages/xylitol-tui ChoicePrompt（替换 editor 槽）完成选择；MUST NOT 调用 prompt_trust_options_stdio 或等价 stderr 数字菜单。
    # verified-by: llmanspec/specs/app-tui-ask/app-tui-ask.feature

    场景: trust-ask-uses-choice-prompt-in-tui
      假如 项目尚未信任
      当 启动需 Trust 闸的路径
      那么 仍走 Trust ChoicePrompt 而非 ask 工具

    场景: choice-slot-replaces-editor
      假如 ChoicePrompt 因 ask 打开
      当 产品 TUI 渲染 editor 槽
      那么 槽为 ChoicePrompt 且标题含 Ask
  @req:r1375
  规则: theme-and-cancel
    Trust ChoicePrompt MUST 使用 Palette::dark()（或等价）choice_prompt_theme；Esc/取消 MUST 视为不信任（deny）；提交 MUST 经 TrustManager 写入持久 store。
    # verified-by: llmanspec/specs/app-tui-trust/app-tui-trust.feature

    场景: trust-choice-theme-and-cancel
      当 读取 trust 选择器主题与取消收口
      那么 主题出自 dark 且取消记为不信任
  @req:r1376
  规则: yolo-after
    项目已信任后产品 TUI MUST NOT 实现逐工具审批 UI；工具默认执行；MUST 保留 hook 扩展点。
    # verified-by: llmanspec/specs/domain-security/domain-security.feature

    场景: hook-point-remains-after-trust
      假如 注册匹配 bash 的 before 拒绝 hook
      当 运行 AgentRuntime 触发 bash
      那么 tool-error 回写且未执行
  @req:r1377
  规则: slash-persist-no-auto-reload
    产品 /trust slash MUST 经 Driver/composition 缝调用 TrustManager（或等价端口）持久化 cwd/parent/deny 决策；MUST NOT 从 app/tui 直接 reach infra::trust；MUST NOT 解冻 Choice/Plate 活板作为本命令唯一路径；写盘成功后本会话 MUST NOT 自动重载项目 skills/MCP/context（用户显式 /reload 或重启除外）。

    场景: trust-slash-persists-without-auto-reload
      当 读取 trust slash 的缝接线
      那么 经 Driver 缝持久化且本会话不自动重载
# re-review(c2826): 复审结论——本 capability 管辖行为不变；分支内改动仅测试基建与可见性再导出（2026-09-28）

# re-review(c2827): 复审结论——本 capability 管辖行为不变；分支内改动为 BDD 场景落地、BDD 测试基建（steps/bindings/驱动旋钮与探针）与可见性再导出（2026-09-28）
    # verified-by: llmanspec/specs/app-tui-trust/app-tui-trust.feature

# re-review(c2837): c2837 编译隔离变更影响本 scope——agent/infra 公开化与 BDD 测试辅助面收敛（纯可见性扩张与测试基建，无行为变化）。场景映射不变量保持；已复核。（2026-10-06）

# re-review(c2838): c2838 intra-doc 链接治理触及本 scope 内源码 doc 注释（纯文档、无行为变化）。场景映射不变量保持；已复核。（2026-10-06）
