# language: zh-CN
功能: 产品 TUI bang Esc
  作为 TUI 用户
  我想要取消卡住的 ! 命令
  以便回到可输入状态且不与 agent Aborted 混淆

  场景: hanging bang Esc 取消
    假定 产品 TUI harness 已启动
    当 提交 hanging bang 并 Esc 取消
    那么 bash 块为 Cancelled 且含 cancelled
    并且 无 Aborted 系统提示
    并且 Driver abort 已被调用
