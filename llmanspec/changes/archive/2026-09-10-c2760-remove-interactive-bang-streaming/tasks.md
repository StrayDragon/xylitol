# Tasks: c2760-remove-interactive-bang-streaming（事件化升级版）

> pre-start。Specs landing 已在绑定分支完成（初版）；方向复议后按新设计改写 spec 与实现。

> **执行注记（apply 完成）**：
> - 事件路径：`session/bash_output`（非 journal，不占 seq）→ remote `handle_frame` → driver `set_bash_run_sink` → TUI bang 循环；in-process 由 driver 生成 `bash_id` 并 relay chunk。
> - 生命周期：`record_bash_start`（running）+ `record_bash_result`（done，同 bash_id）；resume 组合在 TUI 重建（`bash_done_ids` fold）；孤立 running → interrupted。
> - LLM 前缀不变量：`llm_projection` 跳过 `status=Running`；done 投影字节不变。**interrupted 的 LLM 短提示为 spec 的 MAY，本 change 未实现**（UI 组合已做）；后续如需 agent 可见中断提示可另开小变更。
> - 输出上限沿用 `OutputAccumulator`/`DEFAULT_MAX_BYTES`；增量不进 journal。
> - agent bash tool 链路（`XyBashExecutor` chunk 端口 / ToolExecutionUpdate）未动。

## 1. 合约（Specs 二次改写）

- [x] 1.1 改写 `app-tui-bridge` atb9：事件驱动增量追加（sink → pending 块）+ 终态收口；`app-tui-transcript`：实时尾部 + interrupted 组合；`app-tui-host`：非 journal 事件下行与主环事件臂。
- [x] 1.2 [blocked-by: 1.1] `agent-session`（或 `agent-session-store`）补：bash 生命周期 entry（running/done）与 interrupted 组合、LLM 前缀一致性不变量（running 不投影）。
- [x] 1.3 [blocked-by: 1.2] `llman sdd validate c2760-remove-interactive-bang-streaming --strict --no-check`。

## 2. 类型与协议

- [x] 2.1 [blocked-by: 1.3] `EnvMessage::BashExecutionMessage` 增 `bash_id` + `status`（serde default：""/Done）；`BashExecutionStatus` 穷举。
- [x] 2.2 [blocked-by: 2.1] `protocol` 增 `BashChunk { bash_id, seq, data }`；`command.rs` 无需改（sink 非 wire 参数）。
- [x] 2.3 [blocked-by: 2.2] `llm_project` / session 读取组合 helper：running 跳过；孤立 running → interrupted 短提示；旧数据照旧。

## 3. 推送路径

- [x] 3.1 [blocked-by: 2.3] host：`SessionSlot::push_bash_output`（非 journal，`session/bash_output`）；bash 分支生成 `bash_id`、写 running entry、注入 sink、完成写 done entry。
- [x] 3.2 [blocked-by: 3.1] remote：`handle_frame` 收 `session/bash_output` → sink 转发。
- [x] 3.3 [blocked-by: 3.2] in-process：`execute_bash` inherent 化（删 trait 声明/实现）并把 chunk 推 sink；`set_bash_output_sink` 面方法（default no-op）。
- [x] 3.4 [blocked-by: 3.3] TUI `run_interactive_bang`：注入 sink → dispatch → select 收 chunk → `append_bash_output`；Esc/busy/终态语义保持。

## 4. 测试与验证

- [x] 4.1 [blocked-by: 3.4] 单测：字段兼容（无 bash_id/status=done）；组合 helper 三态。
- [x] 4.2 [blocked-by: 4.1] 集成：host 双 entry + 非 journal 事件；remote downlink → sink；TUI harness 增量 + interrupted resume。
- [x] 4.3 [blocked-by: 4.2] `just test-tui` + `just qa`；确认 agent bash tool 链路测试不动。

## 5. 收尾

- [x] 5.1 [blocked-by: 4.3] 交接：`XyBashExecutor` chunk 端口保留（agent 链路）；增量不进 journal；interrupted 仅崩溃后出现。
