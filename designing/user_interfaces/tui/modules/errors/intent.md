# errors

持久错误进错误行：优先 **一行** error 色短文。瞬时硬拒（busy 下直接拒绝提交）的即时反馈走通知条。

- 禁止多行红框墙 / 居中大弹窗挡 scrollback。
- 禁止用 System 指 UI。
- 进 raw mode 前的 CLI 失败直接 `Error:` 退出，不先改终端状态。
