# toast-notice

status / spinner **上方**恰好 1 行通知条。不是滚动提示，不是错误行。

- 整行 warning；可见文案以 `Error: ` 开头。窄宽单行截断，不折第二行。
- 单槽：新通告替换旧通告；约 4s TTL 后自动清除。
- 不占用 status lead；spinner 仍独占 accent。
- 禁止冒充对话正文或队列条。
