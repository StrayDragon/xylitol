# design — c535 Server-on-Driver

## 目标

```text
HTTP/WS 请求
  → server handlers
  → Driver / dispatch(Command)
  → 同 Print 的 ReAct / XyEvent 语义
```

## 迁移

1. `AppState` 以 `tokio::sync::Mutex<InProcessDriver>`（或 RwLock）替换裸 `ReActAgent`。
2. `run` 类接口：`driver.run(prompt)` 流式写回 journal / WS。
3. 模型/会话/导出命令：优先 `dispatch`，不足再扩 Driver。
4. `McpSession` 与 Driver 同生命周期挂在 runtime，不再 `into_agent`。

## 风险

- 锁粒度：长 `run` 持锁会堵其它命令 → 设计「run 中可 abort/steer」的锁策略（可参考单 writer）。
- REST 现有响应形状：对照测试逐步迁。
