---
depends_on: []
branch: sdd/c2240-add-stream-node-timestamps
base_sha: 722ea33317144b801bdd405d2e36e35dc8125820
checkpointed: false
---

# 流式节点墙钟：Thought 只计量思考通道

> **一句话**：assistant 落盘带 `streamTiming` 节点 unix-ms（写一次、用时相减）；Thought {Ns} 只覆盖思考通道，resume 不再用相邻消息墙钟。

## Why

Thought 秒数曾从首个 ThinkingDelta 量到整条 assistant 正文结束。Responses 的 `ThinkingEnd` 还常和 `Done` 一起到，不能当停表点。正确区间是思考通道起止；正文、工具流、MessageEnd 是别的节点。

主线已有思考起止戳与 `thinkingElapsedSecs`。本 change 把合约写进 spec，并把同一套在线打戳扩到同一轮其它合理节点，方便以后任意两点相减。

## What Changes

1. **合约**：Thought {Ns} 的起止墙钟 = 思考通道开始 → 思考通道切走（先到的正文 / 工具意图 / ThinkingEnd）。MUST NOT 计入随后正文或工具流。resume：优先 `thinkingElapsedSecs`，否则节点 ms 相减，再缺则省略；MUST NOT 用相邻条目墙钟冒充思考时长。
2. **落盘 `streamTiming`**（camelCase 附加字段，LLM 投影忽略）：每个发生过的节点写一次 unix-ms。
3. **在线算法**：节点 start 写一次；思考结束写一次（先到的切走信号）；`textEnded` 为最后一次 TextDelta 覆盖；用到时相减。MUST NOT 在 MessageEnd 才回头扫流。

**节点**（同一条 assistant 消息可带；AgentStart / TurnStart 从本 run/本 turn 拷入）：

| 节点 | 何时打 |
|---|---|
| `agentStartedAtMs` | 本 run `AgentStart` |
| `turnStartedAtMs` | 本 ReAct 迭代 `TurnStart` |
| `thinkingStartedAtMs` / `thinkingEndedAtMs` | 首个 ThinkingDelta / 思考通道切走 |
| `textStartedAtMs` / `textEndedAtMs` | 首个 / 最后 TextDelta |
| `toolIntentAtMs` | 首个 ToolCallStart |
| `messageEndedAtMs` | MessageEnd / 落盘 |

**不做**：给每个 XyEvent 加 `ts`（不拓宽线协议）；旧 JSONL 迁移；流式每帧刷新 Thought 秒数。

## Capabilities

- `app-tui-transcript`：Thought 标签与 resume
- `agent-runtime`：落盘 `streamTiming`

## Impact

- Session JSONL assistant 附加 `streamTiming` + 既有 `thinkingElapsedSecs`
- TUI resume 读 elapsed 或 thinking 节点差；删相邻戳回退
- 单测 / 既有 harness；不新开 BDD 步骤

## Test seams

- `StreamNodeClock` / 既有 Thought 时长单测（crate 内）
- ReAct persist 后 session JSON 含节点（agent runtime 单测）
- TUI `rebuild_scrollback_from_travel` resume Thought Ns
