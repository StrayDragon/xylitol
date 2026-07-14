# Design — c669-add-app-tui-scrollback-async-tint

## Decisions（已锁定）

| 主题 | 决议 |
|---|---|
| 架构 | **单环 Mux**：一个 `select!` 扇入 Tick / Input / Resize / Agent / BashChunk / BashDone；**拆除** bang 内层双环 |
| 端口 | **仅新 API**：`BashExecOpts { cancel, chunk_tx: Option<mpsc::Sender<Bytes>> }`；删除 `(command, cancel)` 旧签名，无 shim |
| 通道满 | **try_send + 发送侧合流**（Full 时拼进 coalesce，下次一并送）；禁止阻塞 `send` 背压；禁止「只丢不拼」为默认 |
| 真值分层 | UI chunk = 实时预览；`XyBashResult`（Accumulator）= 落盘/终态 SSOT；Done 时 `finish` 对齐终态 |
| 渲染 | chunk → `append` + dirty；**仅 Tick（~16ms）或 Done** 时 `try_render` |
| UTF-8 | bridge 持不完整码点缓冲，跨 chunk 拼完整字符 |
| 第二 bang | **硬拒绝**（提示 + 恢复 editor）；不排队、不并行 |
| 工具流 | 复用既有 `ToolExecutionUpdate`；不另开 bang 式通道 |
| 取消 | 每 bang 新 `CancellationToken`；Esc → `Driver::abort` → 杀树 + 块内 `(cancelled)`（c665） |

## 扇入形状

```text
HostMuxEvent = Tick | Input | Resize | Agent(XyEvent) | AgentClosed
             | BashChunk(Bytes) | BashDone(Result<XyBashResult>)
```

bang 启动：`begin_bash_block` → `(chunk_tx, chunk_rx)` + 完成 future → 外环 poll；**不** pin 住整个 execute 独占外层。

## 分层

```text
infra/bash_exec     → Bytes chunk（有界 mpsc + coalesce）
runtime_protocol    → BashExecOpts / XyBashExecutor
agent BashExecHandler → 透传 opts；持 cancel
Driver              → execute_bash(..., opts) 应用面入口
app/tui host        → 单环 + 硬拒绝闸
bridge              → append_bash_output / finish_bash_block
```

禁止：infra 碰 UI；bang 进 `XyEvent` 总线；包内产品合流。

## Non-goals

- 多 bang 并行 / 排队
- 旧 API 兼容包装
- 改 ReAct / Track A / computer-use
