# Design：线协议闭集与现行枚举差分

本票只落地**词汇与禁双轨**。不改 `src/protocol/wire` 枚举、不换 HTTP 栈。

## 1. 产品缝（继承 c2300）

```text
client  --Command/Event-->  host dispatcher
carrier 可变；语义不可变
```

REST 可以暂时留着当旧控制面（`ip3`/`server-core` sr2），但 **MUST NOT 再增加产品动词**。真正拆掉 REST 产品语义是监听器票的事。

## 2. 现行枚举 vs 闭集

### Command（已有，保持）

`Prompt` `Abort` `GetState` `SetModel` `CycleModel` `GetAvailableModels` `SetThinkingLevel` `Bash` `Compact` `GetSessionStats` `ExportHtml` `ExportJsonl` `ImportJsonl` `SwitchSession` `Fork` `GetMessages` `GetCommands` `Steer` `FollowUp` `ClearQueue` `Subscribe` `ApproveTool` `AnswerQuestion` `Quit`

旧 spec 名 `Run`/`Cancel`/`SwitchModel`/`ListCommands` = 上表对应变体。本票不改名。

### Event（已有，保持）

`Error` `Response` `TextDelta` `ThinkingDelta` `ToolStart` `ToolEnd` `ToolExecutionUpdate` `AgentEnd` `ModelSelect` `CompactionStart` `CompactionEnd` `Subscribed` `BashResult` `TurnStart` `TurnEnd` `MessageStart` `MessageEnd` `MessageUpdate` `ContextTokenSettlement` `QueueUpdate`

反向 RPC 与 journal 在 **帧**（`ServerHello` / `Ack` / `ResyncRequired` / `ReverseRpc`），不在 `Event` 枚举。本票承认这一分层；不把审批改成 Event 变体。

### 缺口（闭集要能表达；本票不加工）

| 语义 | 现状 | 谁落地 |
|---|---|---|
| host `reload`（MCP / prompt / 技能） | TUI + Driver 方法，无 Command | 实现票（传输面或进线） |
| trust 持久化 | Driver `persist_project_trust` | 同上 |
| 写者 / 只读 | 无变体 | 多会话票 |
| 导出回传内容 | `output_path` 让对端写盘 | 实现票（对齐 c2300 角色） |
| bang 直播增量 | 进程内 `chunk_tx`；Remote 丢掉 | 实现票；MUST NOT 并进 `ToolStart` 流 |
| `ServerHello.protocol: 1` | 现为 `version: String` | 传输面改名；本票禁双语义 |

人 bang 结束态已有 `BashResult`，与工具流已分开。缺的是直播增量。

## 3. 不废止的旧 MUST

| 现行 | 本票 | 谁废止 |
|---|---|---|
| `ip3` REST 信封 | 不删；加「禁止新 REST 产品动词」 | 监听器票 |
| `ip9` Subscribe/Approve 留 WS | 不删；闭集仍把它们当 Command | dispatch 收口票 |
| `server-core` REST 路由 / 锁 | 不动 | c2302 |

## 4. 测试边界

| 测什么 | 怎么测 | 不测 |
|---|---|---|
| 新合约可解析、不与旧 MUST 对打 | `llman sdd validate c2301-update-stable-wire-protocol --strict` | 新 CLI、改枚举单测 |
| 现行枚举仍在 | 现有 `protocol-app` `feature: false` 审查行 | 缺口变体的 serde |
| 运行时无回归 | 现有 `just qa` | 本票不新增可执行 `.feature` |
