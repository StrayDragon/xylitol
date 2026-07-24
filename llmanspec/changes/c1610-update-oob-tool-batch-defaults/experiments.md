# 可先跑实验（不依赖 promote）

## 实验 1 — 慢 I/O 并行证明

**目的**：证明 `barrier_parallel` 墙钟收益（避免「本地 1ms read 看不出并行」）。

**前置**：`tool_batch.mode: barrier_parallel`；Langfuse `api=openai-responses`（与配置一致）。

> bash 是 **Barrier**，三连 `sleep` **不会**并行。用下方 FIFO 慢读（ParallelSafe）。

**终端 A（保持运行）**：

```bash
python3 scripts/xylitol_batch_slow_fifos.py --delay 2
```

**xylitol Prompt（复制）**：

```text
【硬性】同一条助手消息内一次性发出 3 个 read（禁止拆轮、禁止 bash/write/mcp）。
读（limit=5）：
- /tmp/xylitol-batch-demo/slow/a.txt
- /tmp/xylitol-batch-demo/slow/b.txt
- /tmp/xylitol-batch-demo/slow/c.txt
汇报三个 MARK 与体感耗时。
```

**判据**：墙钟 ≈2s（并行）而非 ≈6s（串行）；Langfuse 三 `tool.execute` 同 `barrier_index` 且 start 重叠。

**Langfuse 清单**：

- [ ] 同一 `agent.iteration` 下 ≥3 个 `tool.execute`
- [ ] `tool_batch.mode=barrier_parallel`，同窗 `barrier_index` 相同
- [ ] 三 span `startTime` 重叠（或间隔 ≪ 串行之和）
- [ ] `llm.request.api` 符合配置（修 `c1598` 后）

---

## 实验 2 — End→Done 空隙（抢跑是否值得 · `c1615`）

**目的**：在 **Responses** 下量 `ToolCallEnd` → stream `Done` 的空隙。

**步骤**：

1. 修 `c1598` 后设 `api: openai-responses`，跑「同消息 2–3 个只读 tool + 模型在 tool 后仍可能废话」的任务。
2. 用 provider-trace / Langfuse 时间线对齐：最后一次 ToolCall 相关 mapped End vs Done。
3. 记录空隙 P50/P90；与工具耗时比。

**判据**：空隙经常 ≥ 典型 ParallelSafe 耗时 → 值得 promote `c1615`；否则搁置。

---

## 实验 3 — 多 tool 提示命中率（Ornith）

**目的**：不改代码，先验证「提示能否减少单 tool/turn」。

在项目 `APPEND_SYSTEM.md`（或等价 append）临时加入：

```text
Tool calling policy:
- When several independent read-only tools are needed (read/grep/find/ls), emit ALL of them in the SAME assistant message as multiple tool calls.
- Do NOT narrate "I will call tools in parallel" and then emit only one tool call per turn.
- If you can only emit one tool call this turn, say so explicitly in one short sentence.
- Dependent writes/bash come AFTER the read results of the previous turn.
```

固定用户任务：要求同消息读 3 个小文件。重复 N=10，统计：

| 指标 | 定义 |
|---|---|
| multi_hit | 首个含 tool 的 assistant 消息 toolCall 数 ≥ 3 |
| fake_parallel | 文案含并行但 toolCall 数 = 1 |

命中率可接受后再把文案固化进 `c1605`/`c1610` 默认 fragment。
