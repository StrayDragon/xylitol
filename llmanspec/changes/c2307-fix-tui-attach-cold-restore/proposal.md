---
depends_on:
- c2306-fix-tui-attach-parity
branch: sdd/c2307-fix-tui-attach-cold-restore
base_sha: cd02ae2e8a2f0a18292a0e04a8686fb816b8c30a
checkpointed: false
---

# attach 冷恢复：Host 快照投影，禁止 journal 实况回放

产品 TUI 是 **client**；会话 JSONL / 写者 / journal 在 **Host**。长历史会话的恢复（CLI `--session` 启动、TUI 内 Resume 切换）必须走 Host 消息快照**一次**重建 transcript，而不是把 EventJournal 里的实况回合磁带当 live 事件逐条播放。

## Why

c2306 交付了 mux 常驻与 `subscribe(last_seq)` 断线续传（sr4/w5/w6），但**冷订恢复**与续传共用同一条「journal 全量当 live event」路径：

- Journal 是本进程实况回合磁带（TextDelta、thinking、AgentStart/End）。把它当实况播放 = 长会话恢复时 spinner 转、正文像打字机重演整场生成。
- 客户端没有（也不应有）Host 的 JSONL；transcript 投影源只能是 Host 消息快照。
- 「卡死输入等回放画完」不是修复——仍是逐条当实况步进，只是挡住输入。

现状代码事实（2026-08-21）：attach 侧已有 `get_messages` 一次投影（CLI `--session` 与 Resume 切换都走它），冷订窗口内客户端丢弃 Agent 实况磁带；但该语义**无合约**（specs 只钉了 sr4 续传与 resync），且快照与订阅流之间的时序关系未定义。

## What Changes

- **恢复投影合约**：冷恢复（`--session` 启动、空闲切换会话）用 Host 消息快照**一次**重建 transcript；MUST NOT 把恢复窗内的 journal 实况磁带当 Agent 事件播放。对拍口径：同一长会话，attach 与同进程恢复都应一次出现完整历史、Idle、无假 spinner。
- **冷订 vs 续传分岔**：`subscribe(last_seq=0)` 的恢复语义与 `last_seq>0` 续传（sr4）明确分岔；环形缓冲截断仍走 `session/resync_required`（w5/w6）不变。
- **快照与实况流的时序**：定义快照 unary 与 mux 订阅的应用顺序约束，避免恢复窗内丢增量或重播。

## 开放决策（design 裁决）

- 冷订跳过磁带的归属：Host 侧忽略 vs 客户端丢弃 vs 订阅携带恢复游标。
- 快照形状：现 `get_messages` 是否足够，是否需要 seq 游标衔接快照后的增量。

## Capabilities

- `server-core`：冷订恢复与 sr4 续传的分岔、快照投影的线语义
- `app-tui-host`：attach 恢复投影 vs 实况流、无假 spinner 口径

## Impact

远程 attach（JSONL 只在 Host）秒开长会话：一次出全文、Idle、无假 spinner，体感对齐旧同进程 restore。

## 非目标

- 改 ReAct / 工具闸；改 sr4 断线续传与 w5/w6 resync 既有语义
- 在 TUI 机器上读 Host 的 JSONL 文件
- 分页 / 折叠加载超大会话（先整段一次投影）
- 把 PTY 升级为 `just qa` 硬闸
