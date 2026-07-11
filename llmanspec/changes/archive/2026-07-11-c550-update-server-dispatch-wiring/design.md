# design — c550 Server × dispatch

## 映射

| REST | Command / 策略 |
|---|---|
| POST model | `SetModel` → dispatch |
| POST model/cycle | `CycleModel` |
| POST thinking | `SetThinkingLevel` |
| POST steer / follow-up / queue/clear | `Steer` / `FollowUp` / `ClearQueue` |
| POST bash / compact / export* / import / fork / switch | 对应 Command |
| GET state / models / messages / stats / commands / queue | `Get*` 或保留薄读（若无 Command 则保持 Driver 直读） |
| POST run / DELETE cancel / WS | **不**经 dispatch（与 ce10 一致） |

## 风险

- 响应 JSON 形状需与 c540 RemoteDriver 解析兼容——优先保持 data 字段。
