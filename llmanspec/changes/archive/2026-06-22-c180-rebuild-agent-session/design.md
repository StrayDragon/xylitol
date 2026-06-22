# Design: c180-rebuild-agent-session

## Architecture

AgentSession 作为事件驱动的状态机，拥有以下内部循环：

```
User Input → prompt() → /cmd dispatch? → /skill expand? → /template expand?
                      → build messages array
                      → _runAgentPrompt(messages)
                          → agent.prompt()
                          → agent.continue() loop (handled by AgentLoop)
                          → _handlePostAgentRun()
                              → retry?
                              → compact?
                              → queue drain?
```

## State Machine States

- **Idle**: 无操作进行中，可接受新 prompt
- **Streaming**: LLM 正在生成响应
- **ExecutingTools**: 工具调用执行中
- **Compacting**: 压缩进行中
- **Retrying**: 自动重试进行中
