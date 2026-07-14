# language: zh-CN
功能: 产品 TUI Esc 与 overlay/树
  作为 TUI 用户
  我想要 Esc 在忙碌时中止而不是开树
  以便 overlay 与 abort 语义清晰

  场景: busy 无 overlay Esc 不开树
    假定 产品 TUI harness 已启动
    当 agent 忙碌时按 Esc abort
    那么 EditorSlot 不是 Tree
    并且 Driver abort 已被调用
