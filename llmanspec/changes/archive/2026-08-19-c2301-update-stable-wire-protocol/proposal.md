---
depends_on:
- c2300-update-cs-capability-split
branch: sdd/c2301-update-stable-wire-protocol
base_sha: 7acd591a41afffa741a7a1ea199ec53701e3aa6b
checkpointed: true
checkpoint_sha: 7acd591a41afffa741a7a1ea199ec53701e3aa6b
---

# 稳定线协议闭集

Command/Event 是 client ↔ host 的唯一产品真源。载体可变，语义不可双轨。角色归属见已归档 c2300。证据：`c2280` `03-paths`。

## Why

现 wire 未覆盖全部产品语义，旁路与「进程内 vs 远程」会分叉。本票钉**闭集与禁双轨**；不换 HTTP 栈、不增运行时枚举变体。

## What Changes

### 产品真源

- 一切产品语义都是 Command 或 Event。embed 与 attach 同一套类型。
- 0.0.1 前禁双语义：MUST NOT 为远程再开平行 REST 产品动词。现行 REST 信封（`ip3`）在监听器收口前仍可存在，但 MUST NOT 再扩展。
- 面本地不进协议：剪贴板（含 OSC 52）、TTY、`$EDITOR`、键位、绘制。

### 闭集（词汇；本票不改 Rust 枚举）

现行已有、必须保持：运行 `Prompt`、中止 `Abort`、模型 / thinking、会话、压缩、steer / follow-up / 队列、`Subscribe`、`ApproveTool` / `AnswerQuestion`、人 `Bash` + `BashResult`、工具 `ToolStart` / `ToolExecutionUpdate` / `ToolEnd`、文本 / thinking 增量、回合生命周期、`QueueUpdate`、`ResyncRequired`（帧）。

缺口（闭集必须能表达，变体落地随后续实现票）：host 侧 `reload`、trust、写者/只读状态、导出回传内容（相对现行 `output_path`）、人 bang 直播增量（相对现行结束态 `BashResult`）。缺口 MUST NOT 用 REST 或第二套动词冒充。

### 握手与会话

- 每条消息归属 session。`Subscribe { session_id, last_seq }` 续传；缓冲满则 `ResyncRequired`。
- 握手带协议版本；对不上则断开，不降级。现行 `ServerHello.version` 在改名为 `protocol` 前视为同一字段，MUST NOT 两套握手版本语义并存。
- 反向 RPC：host 问客户端；第一应答生效（现行帧已有）。
- 一 session 一写者：协议闭集必须能表达写者/只读；尚未有变体时同进程路径视为已满足（c2300 `la-cs5`）。

## 非目标

字段/命名/codec 切片（含 `Run` vs `Prompt` 旧 spec 名、`version` → `protocol` 改名）。编码不二进制化。不改运行时 `Command`/`Event` 枚举。多会话组合根、进线 CLI、符合性闸、ACP。不废止 `ip3` REST 信封、`ip9` 传输专属变体分家（留给监听器/dispatch 收口）。

## Capabilities

- `protocol-app`：单一产品真源、面本地排除、bang/工具分流、握手版本、闭集缺口。

## Impact

- `server-core` 的 REST 路由 / 锁 / 帧实现本票不改。
- 后续 c2302 传输面必须按本票闭集走 WS 承载 Command/Event，禁止新 REST 产品动词。
