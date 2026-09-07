---
depends_on: []
branch: sdd/c2600-add-obs-dual-session-identity
base_sha: a7e82a8c5fa452edbc86b8819e8ae8b0c4bf7f12
checkpointed: true
checkpoint_sha: 635f91cb84a492cd6230faa4703b77e6526869e6
---

# 观测拆开「xylitol 会话」与「发给 LLM 的会话身份」

## Why

### 真值词：session 里有 entry，entry 是树/链表上的节点

产品对象只有这些（穿越 / 世界线 / 书签都只是理解用的比喻，不进键名）：

```text
session          一本对话（一份 JSONL，header.id，用户能切换）
  └── entry      树上的节点（消息等）；entry.parent_id 连的是**上一条 entry**，不是父 session
fork             新建一个 session，把父 session 里到某条 entry 为止的节点拷过来
```

今日观测只有 `langfuse.session.id` = 这个 xylitol session 的 UUID。三件不同的事挤在这一个值里：

1. **现在在哪本 session**（产品 Session 视图、切换、命名）
2. **这次请求在 LLM / 网关上算哪次会话**（无状态 API：我们把前缀再发一遍；网关 header / 缓存键以后可能跟 xylitol session id 脱钩）
3. **这本 session 是从哪本、哪条 entry 长出来的**（父 session + 切点节点）

### 为什么要拆（一个 fork 例子）

会话 **A** 里有一串 entry。用户点中节点 `u6` 做 fork，得到会话 **B**。

- 产品里 A、B 是两本 session → Langfuse Session 必须仍按 **B** 切开，不能揉进 A。
- LLM 无状态：B 的下一轮请求带的是与 A 共享的那段 entry 前缀。c2620 可能让「发给网关的会话身份」继续跟这段前缀走，而不是跟新 session id 走。那是**另一条轴**，不能占用 `langfuse.session.id`。
- 树边：B 来自 session A、切在 entry `u6`。没有这两项，Langfuse 里 B 是孤岛。`ForkPosition::Before` 时 `u6` 根本不拷进 B，所以切点必须写在 B 的 header 上。

本刀只**钉字段**，且观测键的值 MUST 是事实：没向 LLM 通道呈报会话身份时，**不写** `llm_gateway_session_id`（禁止用 `xylitol.session.id` 占位）。今日 OpenCode 请求会带 `x-opencode-session`，该键才等于呈报值。c2620 只改呈报策略，不改「没呈报就不写键」。

本刀之后，B 上一次 generate：

| 键 | 值 | 意思 |
|---|---|---|
| `langfuse.session.id` | B | 产品 Session = 这本 xylitol session |
| `xylitol.session.id` | B | 同上，显式 xylitol 键，不靠 Langfuse 厂商键兼差 |
| `xylitol.session.llm_gateway_session_id` | 实际呈报值 | 发给 LLM 通道的会话身份（c2620 再填策略；未呈报则省略键） |
| `xylitol.session.parent_session_id` | A | 父 **session**（禁止叫 `parent_id`，那是 entry 链） |
| `xylitol.session.fork_at_entry_id` | u6 | 父 session 里作为切点的那条 **entry** |

无父的新 session 不写后两个键。

## What Changes

- 观测键（session / entry 词，不用 bookmark / worldline）：
  - `xylitol.session.id`：当前 xylitol session
  - `xylitol.session.llm_gateway_session_id`：仅当本次 LLM 请求实际向通道呈报了会话身份时写出（值 = 呈报值；今日 OpenCode 为 `x-opencode-session`）
  - `xylitol.session.parent_session_id`：父 session
  - `xylitol.session.fork_at_entry_id`：切点 entry
- `langfuse.session.id` **永远等于** `xylitol.session.id`，禁止改成 `llm_gateway_session_id`
- 子本 header 可选 `forkAtEntryId`，与已有 `parentSession` 成对；旧文件无键则省略切点属性。不 bump `SESSION_VERSION`，不改 entry 拷贝 / COW 写路径，本刀不补生产 `branchSummary`
- 禁止用单一 `session.id` 混指 xylitol session 与 LLM 会话身份；不保留旧属性别名

## Capabilities

- `infra-otel`：新 req **otel26**（勿复用 c2610 的 otel24）。otel6 语义不变（`langfuse.session.id` = 当前 session UUID）；显式 `xylitol.session.id` 放在 otel26，不改已锁 otel6 句面
- `agent-session-store`：fork 写入 `header.forkAtEntryId`（= `at_entry_id`）；非 fork 创建省略
- `package-ai-bridge`：只扩观测快照字段，**不**新开 pab req

## Impact

- Langfuse 里一本 xylitol session 仍是一个 Session；分析用 `xylitol.session.*` 看 LLM 身份与 fork 树边
- 只认 `langfuse.session.id` 的 dashboard 行为不变（仍是 xylitol session）
- 不打开 `prompt_cache_key`；不改 fork 的 entry 拷贝

## Further Notes

切片链：A=`c2590-fix-obs-session-per-generate`（已归档）→ 本刀 → B=`c2620-add-provider-session-key-policy` 填满 `llm_gateway_session_id`。原伞 `c2580-add-session-identity-split` 已拆除。架构文里的「书签」仍是同一对象的旧称，本刀观测键不再用 bookmark。

## Open Questions

- [x] 切点语义：fork 的 `at_entry_id`（含 `Before` 时未拷进子本的那条 entry），不是拷贝路径叶。
- [x] 切点落盘：`SessionHeader.forkAtEntryId`；观测键 `xylitol.session.fork_at_entry_id`。
- [x] live spec：`infra-otel`（otel26）+ `agent-session-store`；ai-bridge 只扩快照。
- [x] 词汇：产品对象是 session + entry 节点。比喻不进键名。
- [x] 四键：`xylitol.session.id` / `llm_gateway_session_id` / `parent_session_id` / `fork_at_entry_id`。`langfuse.session.id` == `xylitol.session.id`。禁止 `parent_id`。
