# language: zh-CN
功能: 产品 TUI abort 与迟到流事件
  作为 TUI 用户
  我想要 Esc 真正中止当前轮
  以便不会继续浪费 token 或复活正文

  场景: busy Esc 后迟到 TextDelta 不得复活
    假定 产品 TUI harness 已启动
    当 agent 忙碌时按 Esc 且在 drain 前注入迟到 TextDelta
    那么 UI 含 Aborted 系统提示
    并且 无 assistant 正文含 SHOULD_NOT_APPEAR
    并且 Driver abort 已被调用
