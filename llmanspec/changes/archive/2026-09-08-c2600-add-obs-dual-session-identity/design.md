# Design: c2600 观测双会话 id

## Vocabulary

只使用产品里已有的对象名：

| 词 | 是什么 | 不是什么 |
|---|---|---|
| **session** | 一本对话（JSONL / `header.id` / 用户切换） | Langfuse 厂商字段；网关 header |
| **entry** | session 内树上的节点 | session 本身 |
| **`entry.parent_id`** | 上一条 entry | 父 session |
| **`header.parentSession`** | 父 session | entry 链 |
| **`forkAtEntryId`** | fork 时选中的那条 entry（`at_entry_id`） | 拷贝路径的最后一条 |

书签 / 世界线 / 时间线：理解用，不进属性名。

## Decision

两套 session 身份都是一等属性，不靠 `langfuse.session.id` 兼差：

| 观测键 | 含义 | `langfuse.session.id` |
|---|---|---|
| `xylitol.session.id` | 当前 xylitol session | **同一值** |
| `xylitol.session.llm_gateway_session_id` | 本次 LLM 请求实际呈报的通道会话身份；未呈报则省略 | 不占用 |

观测 MUST 记事实：禁止用 `xylitol.session.id` 给 `llm_gateway_session_id` 占位。今日 OpenCode 呈报 `x-opencode-session` 时，该键等于呈报值（可与 xylitol session id 相同，因为线上就是这个值）。c2620 未落地时非 OpenCode 路径 MUST NOT 写该键。

树边（仅子 session，header 带 `parentSession`）：

| 观测键 | 盘上字段 |
|---|---|
| `xylitol.session.parent_session_id` | 已有 `parentSession` |
| `xylitol.session.fork_at_entry_id` | 新可选 `forkAtEntryId` |

禁止观测键叫 `parent_id`（与 entry 链撞名）。旧文件无 `forkAtEntryId` → 省略切点属性。无旧键兼容窗。

## Non-goals

- 并发槽（A / c2590）
- OpenCode header / `prompt_cache_key` 算法（B / c2620）
- 改 COW 写路径、bump `SESSION_VERSION`、生产 fork 写 `branchSummary`
- 改术语表里的「书签」旧称（架构文可后置对齐）
