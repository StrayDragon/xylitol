# Design: 交互 bang 输出事件化与中断残留

> Designed / pre-start。c2710 已归档（`Command::Bash` 执行器已存在）。

## 1. 目标与不变量

1. **单入口**：交互 bang 只经 `dispatch(Command::Bash)`；`XyDriver` 无 `execute_bash`。
2. **输出事件化**：增量经非 journal 会话下行（`session/bash_output`）→ driver 面级 sink → TUI pending 块；本机 / remote 同源。
3. **输出上限**：沿用 `OutputAccumulator` + `DEFAULT_MAX_BYTES` 溢出落临时文件；终态 entry 带截断视图 + `full_output_path`。
4. **前缀一致性（硬不变量）**：LLM 可见 bang 投影 = done entry 的稳定投影；running entry 不产出 LLM 投影；resume 重建历史与实时增量历史在 bang 部分字节一致。
5. **中断残留**：start(running) + finish(done) 双 entry；无 done 的 running 在 resume/transcript 组合为 interrupted。

## 2. 类型与协议

### 2.1 `EnvMessage::BashExecutionMessage` 扩展

```rust
pub enum BashExecutionStatus { Running, Done }  // serde: "running" | "done"，default Done

BashExecutionMessage {
    bash_id: String,                 // 新；serde default ""（旧数据）
    command: String,
    output: String,
    exit_code: Option<i32>,
    cancelled: bool,
    truncated: bool,
    full_output_path: Option<String>,
    exclude_from_context: bool,
    status: BashExecutionStatus,     // 新；default Done（旧数据 = 完成）
}
```

- **running entry**：`output: ""`、`exit_code: None`、`status: Running`（轻量，开始即写）。
- **done entry**：现有结果字段 + `status: Done`。
- 旧 JSONL 无新字段 → serde default → Done，行为不变。

### 2.2 输出事件（非 journal 下行）

先例：`SessionSlot::push_resources` → `RpcMessage::ServerRequest { method: "session/resources" }`（`push_resources_is_not_journaled` 测试钉住「不占 journal seq」）。

- 新方法：`session/bash_output`，payload `{ session_id, bash_id, seq, data }`。
- `data`：UTF-8 lossy 文本（终端主体）；跨块 UTF-8 残余由消费端（TUI `bash_utf8_pending`）处理。
- **不占 journal seq**（与 `session/resources` 同族）。

## 3. 执行与推送路径

```text
TUI:    set_bash_output_sink(tx) → dispatch(Command::Bash) → select 收 chunk → append_bash_output
                                         │
host:   record bash start(running) ──▶ sink 注入 = slot.push_bash_output(bash_id)
        driver.execute_bash ──(chunk)──▶ sink ──▶ session/bash_output（广播，非 journal）
        finish ──▶ record_bash_result(done) + unary 返回结果
in-process(嵌入): sink 由 TUI 注入；execute_bash 直接喂 tx（无 host 也同源）
remote: downlink 收 session/bash_output → 转 sink（TUI 注入的 tx）
```

### 3.1 driver 面能力（`XyDriver`）

```rust
/// 交互 bang 输出事件的消费端（面接线；非会话操作，非 wire 参数）。
fn set_bash_output_sink(&mut self, sink: Option<tokio::sync::mpsc::Sender<BashChunk>>) { /* default: no-op */ }
```

- in-process：`execute_bash` 内部把累积 chunk 以 coalesce 方式推 sink（复用现有 `emit_chunk` 策略）。
- remote：`handle_frame` 收 `session/bash_output` → 若 sink 已注入则推；未注入则丢弃（非 journal，无重放义务）。
- TUI 在 dispatch 前注入、完成后清除。

### 3.2 TUI bang 循环（`effects/bang.rs`）

- 保留：dispatch 单一入口、Esc → `Command::Abort`、busy 硬拒绝、pending/终态渲染。
- 恢复：select 臂接收 sink mpsc → `session.append_bash_output(&chunk.data)`（bridge 逻辑保留）。
- dispatch future 与 sink 分离（不同通道），无借用冲突。

## 4. 中断残留组合

### 4.1 写入

- `record_bash_start(store, bash_id, command, exclude_from_context, session_id)`：写 running entry。
- 完成仍走 `record_bash_result`（加 `bash_id` + `status: Done`）。
- 取消（Esc）也是 finish（`cancelled: true`）。

### 4.2 读取组合（session 读取层统一）

`load_entries` 后消费方（transcript 重建 / LLM 上下文）按 `bash_id` 组合：

| 情形 | transcript | LLM 投影 |
|---|---|---|
| running + done | 只显示 done | 只投影 done |
| running 无 done（崩溃） | **interrupted**（command + 中断提示 + path 若在） | 一条短提示（`[interrupted] $ cmd`，`exclude_from_context` 时跳过） |
| 旧数据（无 bash_id/status） | 正常 done | 正常投影 |

组合逻辑放 `protocol::session` 一个穷举 helper（`fold_bash_lifecycle` 或等价），transcript 与 `llm_project` 共用，避免两套。

### 4.3 前缀一致性论证

- 实时路径：done entry 进入内存历史 → `llm_projection()`（`$ cmd\noutput`）。
- resume 路径：JSONL 读回 running + done → 组合 → 只剩 done → 同一 `llm_projection()`。
- 字节一致：done entry 字段（command/output/exit_code/…）不变；running 不投影。
- interrupted 仅存在于无 done 的崩溃会话——该会话此前没有稳定的 API 前缀（进程已死），不构成破坏。

## 5. 验证

- 单测：`BashExecutionMessage` 序列化兼容（无新字段=done）；组合 helper（running+done / 孤立 running / 旧数据）。
- BDD/集成：
  - host：bash 执行推送 `session/bash_output`（非 journal：不占 seq）+ start/done 双 entry 落盘。
  - remote：downlink → sink 消费（`remote` 集成测试）。
  - TUI harness：sink 驱动的 pending 块增量 + 完成后收口；resume 组合 interrupted。
- `just qa`；确认 agent bash tool 链路（`XyBashExecutor` chunk 端口 / `ToolExecutionUpdate`）测试不动。
