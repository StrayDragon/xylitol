# c87-add-rpc-mode Tasks

- [ ] 定义 JSON-RPC 2.0 请求/响应/通知结构体
- [ ] 实现 stdio 读写循环（tokio::io::stdin/stdout）
- [ ] 实现 agent/prompt 方法（接收用户输入，启动 agent session）
- [ ] 实现事件流 → JSON-RPC 通知转换（TextDelta → agent/text_delta, ToolCall → agent/tool_call）
- [ ] 实现 agent/cancel 方法（中断当前执行）
- [ ] 集成 CLI 分派（--mode json）
- [ ] 编写测试（mock stdin/stdout，协议解析）
- [ ] `just fmt && just lint && just test`
- [ ] `llman sdd validate c87-add-rpc-mode --strict --no-interactive`
