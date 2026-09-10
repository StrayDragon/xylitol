---
depends_on: []
rules_edit_acked: true
branch: sdd/c2760-remove-interactive-bang-streaming
base_sha: b2604d2845b4bdb5f8c2de881dd5073e442e473f
checkpointed: true
checkpoint_sha: b2604d2845b4bdb5f8c2de881dd5073e442e473f
---

# 交互 bang 输出事件化与中断残留

> id 保留自初始草案（原方向为「去流式」）；复议后本 change 交付输出事件化 + 中断残留，避免重建分支/specs 骨架。

## Why

用户主动触发的交互 bang（`!` / `!!`）当前的实时输出走进程内 mpsc 参数（`execute_bash(..., chunk_tx)`，c669）：

- 只在 in-process 生效；产品 TUI 默认 attach Host，`XyRemoteDriver` 忽略 `chunk_tx`——**watch 等长命令在产品面完全不可观测**（只有 pending 块，直到结束/取消）。
- 该参数让 `XyDriver` 在 c2710 后仍留 `execute_bash` 会话操作例外；TUI 行为随 driver 类型分叉。
- 命令中断残留（TUI/host 崩溃）时 session JSONL 无记录——resume 看不到「曾有命令未完成」。

产品定调是「服务端执行、完成返回结果」——**执行模型不变**；本 change 把「输出增量」从进程内参数迁到**非 journal 会话事件下行**，并补齐中断残留的生命周期记录。

## What Changes

- **执行入口**：`XyDriver` 删 `execute_bash`；交互 bang 唯一入口 `dispatch(Command::Bash)`（执行器无本地通道参数）。
- **输出事件**：host 以非 journal 下行（`session/bash_output`，先例 `session/resources`）推送增量；driver 提供面级 sink 注入（`set_bash_output_sink`），in-process 执行路径与 remote downlink 都喂同一 sink；TUI 从 sink 消费并更新 pending 块（保留跨块 UTF-8 处理）。
- **输出上限**：沿用 `DEFAULT_MAX_BYTES` + `OutputAccumulator` 溢出落临时文件；最终 entry 携带截断视图 + `full_output_path` 路径 tip。
- **中断残留**：JSONL append-only → 双 entry：
  - start：轻量 `bashExecution { bash_id, command, status: running, output: "" }`
  - finish：结果 entry `{ bash_id, status: done, output(截断), exit_code, cancelled, truncated, full_output_path }`
  - resume/transcript：同 `bash_id` 有 done → 只显示 done；running 无 done → 组合渲染 **interrupted**（全文路径若在）。
- **LLM 前缀一致性（硬不变量）**：LLM 可见 bang 投影 = done entry 的稳定投影；running entry 不产出任何 LLM 投影；resume 重建的历史与实时增量历史在 bang 部分字节一致。interrupted 仅崩溃后出现，投影一条短提示（`!!` 仍排除）。
- **Specs**：`app-tui-bridge` atb9 改为「事件驱动增量 + 终态」；`app-tui-transcript` 改为「实时尾部 + 中断组合」；`app-tui-host` 主环/Tick 恢复事件臂语义；`agent-session`（或相关 capability）补「bang 生命周期 entry + interrupted 组合」与前缀一致性要求。

非目标：

- 不改 agent bash tool 链路（`XyBashExecutor` chunk 端口、ToolExecutionUpdate 照旧）。
- 不改 `Command::Bash` 的同步 unary 执行模型与 Esc 取消语义（`Command::Abort`）。
- 不做「断线期间的增量 journal 重放」（增量不进 journal；终态由 finish entry 保障）。
- 不做输出增量进 journal 的检查点（watch 膨胀）。

## Capabilities

- `app-tui-bridge`：事件驱动增量追加（sink → pending 块）+ 终态收口。
- `app-tui-transcript`：实时尾部渲染 + resume interrupted 组合。
- `app-tui-host`：非 journal 事件下行与主环事件臂。
- `agent-session`（或 `agent-session-store`）：bash 生命周期 entry（running/done）与中断组合、前缀一致性。

## Impact

- 产品 TUI（remote）交互 bang 获得实时尾部输出（与 in-process 同源）。
- 崩溃/断线后 resume 可显示 interrupted 残留与路径 tip。
- `EnvMessage::BashExecutionMessage` 增 `bash_id`/`status`（旧数据无字段 = done，兼容）。
- `XyDriver` 无会话操作方法；trait 例外关闭。
