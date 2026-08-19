---
depends_on:
- c2306-fix-tui-attach-parity
---

# attach 冷恢复：Host 快照投影，禁止 journal 实况回放

产品 TUI 是 **client**；会话 JSONL / 写者 / journal 在 **Host**。`--session` 恢复长历史时，attach 用 `last_seq=0` 订阅，把 EventJournal 里的 AgentStart / 文本增量 / AgentEnd **当本轮实况**灌进 TUI，spinner 会转、正文会像打字机从头播一遍。本票只记方向，落实走 propose。

## Why

c2306 把 mux 常驻和 `subscribe(last_seq)` 续联做对了，但冷恢复和断线续传共用同一条「journal 全量当 live event」路径。

- TUI **没有**（也不该有）本机 JSONL。快照必须来自 Host unary（`get_messages` / 等价 entries），不是 TUI 读盘。
- Journal 是本进程 **实况回合磁带**（delta、thinking、AgentStart），不是 transcript 投影源。
- 用「卡死等到全部画完」藏回放是错的：仍然在逐条当 live 步进，只是挡住输入；长会话更慢。

## What Changes（草案，未拍板）

- 冷 attach / `--session` / 空闲切会话：用 Host 消息快照 **一次**重建 transcript（现有 `apply_cli_restored_session` 一类），**MUST NOT** 把历史 journal 当实况 Agent 事件播放。
- `subscribe`：空闲恢复跳过历史磁带（例如订到当前 `max_seq`，或客户端丢弃恢复窗内的 Agent* 实况）；journal 重放 **仅**留给回合中途断线（`w6` / `last_seq+1`）。
- Loading 中间态：仅当 Host 快照 unary 本身慢（大 JSONL）时 MAY 加；**禁止**用 loading 掩盖 live replay。
- 对拍：同一长会话，InProcess 与 HttpWs 都应一次出现完整历史、Idle、无假 spinner。

## 开放决策（propose 时深挖）

- 跳过回放：Host 侧 `subscribe` 对冷订忽略磁带 vs 客户端忽略 vs 专用 `resume` 语义。
- 与 `session/resync_required` / `w6` 全量再订如何分岔（冷恢复 vs 环形缓冲截断）。
- 快照形状：现有 `get_messages` 是否够，要不要带 `seq` 游标以免快照后丢增量。
- 大会话：分页 / 折叠 / 先投影 leaf 路径再补，是否本票。

## Capabilities（拟）

- `app-tui-host` / `app-tui-input`（恢复投影 vs 实况流）
- `server-core` / `protocol-app`（subscribe 冷订 vs 续传）

## Impact

远程 attach（JSONL 只在 Host）也能秒开长会话，体感对齐旧同进程 restore，而不是重演整场生成。

## 非目标

c2306 已交付的 MCP 头卡 / 队列条 / 定稿闸 / mux 常驻；c2315 一条命令；在 TUI 机器上读 Host 的 JSONL 文件；把 PTY 升成 `just qa` 硬闸。
