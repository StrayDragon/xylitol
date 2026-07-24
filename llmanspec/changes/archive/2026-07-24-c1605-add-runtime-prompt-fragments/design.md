# Design: c1605 + c1610（同分支）

## c1605

```text
XyBatchMode ──► fragments_for_batch_mode() ──► SystemPromptOpts.runtime_policy_fragments
                                                      │
                                                      ▼
                                            build_system_prompt → <runtime_policy>
```

- Fragment id `tool_batch.barrier_parallel`：仅 mode=BarrierParallel 时注入。
- Session 持有 `runtime_fragment_ids`：id 集合不变则 sync no-op（构造后再 `set_tool_mode(同模式)` 不重复写）。
- Sequential：不注入（或未来可加「串行确认」片段——本 change 不做）。
- 单测：mode 切换启停；同 id 不重复；APPEND_SYSTEM 仍可叠加且出现在 policy 段之前。

## c1610

- `ToolBatchMode` / `XyBatchMode` / Session 缺省 → `BarrierParallel`。
- `rc26` / `ar27` 文案与 BDD：未配置 = 并行窗重叠；显式 `sequential` 仍串行（配置单测 + 保留 ar28 族）。
- 默认 fragment 文案（实验 3）：

```text
Tool calling policy:
- When several independent read-only tools are needed (read/grep/find/ls), emit ALL of them in the SAME assistant message as multiple tool calls.
- Do NOT narrate parallel tool use while emitting only one tool call per turn.
- If you can only emit one tool call this turn, say so in one short sentence.
- Dependent writes/bash come after prior-turn read results.
```

- 移除开发仓临时 `.xylitol/APPEND_SYSTEM.md`；`tool_batch.mode` 可从开发 yaml 删除（默认已并行）。
