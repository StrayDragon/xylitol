# Design: RPC Mode (JSONL over stdio)

对齐 pi `modes/rpc/`（jsonl.ts / rpc-mode.ts / rpc-client.ts / rpc-types.ts）。

## 决策

1. **stdio 行协议**：stdin 逐行读 JSON 命令，stdout 逐行写 JSON 事件/响应，每行 flush。stderr 不参与协议（仅日志）。对齐 pi jsonl.ts。
2. **`RpcCommand` 全覆盖**：对齐 rpc-types.ts 全部命令（见 proposal 列表）。未知命令返回 `{ type: "error", id, message }`，不崩溃。
3. **id 关联**：命令与响应都带可选 `id`；事件流（无对应请求的流式事件）不带 id。客户端可用 id 匹配请求-响应。
4. **事件桥接**：订阅 `AgentEvent` 流 → 映射为 `RpcEvent`（TextDelta/ToolStart/ToolEnd/Compaction/ModelSelect/AgentEnd 等）。复用 c80 的 AgentEvent 枚举。
5. **信号处理**：SIGINT/SIGTERM → 触发 `agent_loop.abort()` → emit abort 事件 → 干净退出（exit 0）。避免僵尸子进程（bash/工具）。
6. **与 CLI 解耦**：`--rpc` 在 `interface/cli/mod.rs` 早路由到 `run_rpc_mode`，不走 print 路径。RPC 内自建 AgentSession（复用现有 new()）。
7. **集成测试**：用管道驱动（fake provider），避免依赖真实模型。验证 prompt→TextDelta→AgentEnd 全链路。

## 不做

- 不实现 rpc-client（pi 的 TS 客户端；Rust 侧无对等需求，外部集成方自写）。
- 不做 ACP（独立 feature `infra-acp`，仅 config 占位，不在本范围）。
