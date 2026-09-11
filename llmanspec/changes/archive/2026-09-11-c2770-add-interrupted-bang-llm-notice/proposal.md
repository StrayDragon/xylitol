---
depends_on: []
branch: sdd/c2770-add-interrupted-bang-llm-notice
base_sha: c4d517e21bd7759521974824309bab6318964b21
checkpointed: true
rules_edit_acked: true
checkpoint_sha: c4d517e21bd7759521974824309bab6318964b21
---

# interrupted bang 的 LLM 短提示

## Why

c2760 引入交互 bang 的 running/done 生命周期与中断组合：

- UI（resume transcript 重建）已把「孤立 running（同 `bash_id` 无 done）」渲染为 interrupted；
- 但 LLM 历史投影跳过 `status=Running`（前缀一致性不变量），因此崩溃中断的命令对 agent **完全不可见**——agent 不知道上一轮有命令中途死亡，可能误判环境状态（例如认为某个 `!serve` 还在跑）。

c2760 spec（`agent-session` as-bang1）把 interrupted 的 LLM 短提示定为 **MAY**；本 change 落地它。

## What Changes

- LLM 历史折叠：对「孤立 running（同 `bash_id` 无 done）」产出**一条简短 interrupted 提示**（如 `[interrupted] $ <command>`），并尊重 `exclude_from_context`（`!!` 不投影）。
- 前缀一致性：提示只在「该会话没有对应 done」的稳定态出现；同一会话重复构建上下文时字节稳定。有 done 的 running 仍完全跳过（done 投影不变，正常会话前缀零影响）。
- 折叠位置：`build_session_context` / 历史播种路径（SessionEntry → AgentMessage）统一做一次 bash 生命周期折叠，避免 `project_for_llm` 单遍无法跨行关联 `bash_id`。
- spec：`agent-session` as-bang1 的 interrupted 子句从 MAY 升为 MUST（细化格式与排除规则）；如组合语义需要跨 store 说明，同步 `agent-session-store`。

## 非目标

- 不改 UI 的 interrupted 渲染（c2760 已做）。
- 不把中断提示写回 session JSONL（仅投影层组合）。
- 不改 running/done 双 entry 写入与 `session/bash_output` 事件路径。

## Capabilities

- `agent-session`：as-bang1 投影子句细化（interrupted 提示 + 前缀稳定）。
- 可能 `agent-session-store`：组合语义备注（无新约束则不动）。

## Impact

- agent 在恢复被中断的会话时能看到「命令已中断」的元信息，避免误判环境。
- 投影字节：仅崩溃会话多一条提示；正常会话零变化（前缀不破坏）。
