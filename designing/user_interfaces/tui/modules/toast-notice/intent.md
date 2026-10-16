# toast-notice

status / spinner **上方**恰好 1 行通知条。不是滚动提示，不是错误行。

- 整行 warning；可见文案带 `Error: ` 前缀（固定词见 TUI信息呈现词汇「通知条」）。窄宽单行截断，不折第二行。
- 单槽：新通告替换旧通告；TTL 到期自动清除（时长以代码为准）。
- 不占用 status lead；spinner 仍独占 accent。
- 禁止冒充对话正文或队列条。
