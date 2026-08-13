# Session 460ad16e：折叠条虚构复刻

> 路径：`$HOME/.xylitol/sessions/460ad16e-874f-418f-8ca0-dabc58f89320.jsonl`（779K，2026-08-03）。按 c1761 切簇规则 + 当时 `count_middles` 复刻，**不是**把整份 JSONL 检进仓库。

## 根因（代码）

`count_middles`：Thinking/Ask/Compaction 不计类目；空类目且 middles 非空时 `files = 1`。未知/`mcp:*` 走 else → 也当 file。于是头行 `Explored 1 file`。

## 与截图同一轮

User `cool 总结下`（ui 序约 209）：

| 条目 | 事实 |
|---|---|
| tool / read | **0** |
| thinking / assistant 正文 | **0** |
| compaction | tokens **101494** |
| Error | 两条 provider `exceed_context_size`（不进信封） |

当时展开信封可见：

```text
▾ Worked for …
▸ Explored 1 file          ← 虚构
▸ [compaction] Compacted from 101,494 tokens
error: …
```

本票目标：

```text
▾ Worked for …
▸ [compaction] Compacted from 101,494 tokens
error: …
```

## 同会话其它虚构簇（7 段活动里 8 个假 Explored）

| 段 | 用户 | 假簇实际 |
|---|---|---|
| seg-0 | hi | 仅 thinking |
| 工具演示 / python 重写 | （各段开头） | 助手正文前的 thinking-only |
| 5000 行 todo | 段末 | 仅 compaction |
| 第二次 `cool 总结下` / hello | | 仅 thinking |
| python 段中 | MCP `lspz:*` 独簇 | 被当成 Explored 1 file |

真实 `ls`/`read` 的 Explored、真实 `write`/`edit` 的 Edited、真实 `bash` 的 Ran **保留**；只去掉占位与 MCP 误伤。

## 手测

resume 该 session（或同等 compaction+error 夹具）：展开最近信封，头行不得出现 Explored。`just qa` 不跑此 JSONL。
