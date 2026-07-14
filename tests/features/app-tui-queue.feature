# language: zh-CN
功能: 产品 TUI 队列键位
  作为 TUI 用户
  我想要在忙碌时 steer / follow-up / 取回队列
  以便插话与改队列符合 Driver 契约

  场景: busy Enter 触发 steer
    假定 产品 TUI harness 已启动
    当 agent 忙碌且编辑器有文本时按 Enter
    那么 Driver steer 被调用一次

  场景: busy Alt+Enter 触发 follow-up
    假定 产品 TUI harness 已启动
    当 agent 忙碌且编辑器有文本时按 Alt+Enter
    那么 Driver follow_up 被调用一次

  场景: Alt+Up 清空双队列
    假定 产品 TUI harness 已启动
    并且 已入队 steer 与 follow-up
    当 按 Alt+Up
    那么 Driver 双队列已清空
