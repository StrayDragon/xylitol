---
depends_on:
- c2304-add-conformance-gate
blocks:
- c2315-add-loopback-host-tui
branch: sdd/c2306-fix-tui-attach-parity
base_sha: 977650e00063f87948e75879c05e903a53ea2b56
checkpointed: true
checkpoint_sha: 9eee3e5c9b8d0e4b4b7c3871f79cda8f8314f2bf
---

# 产品 TUI attach 对拍旧同进程体验

c2304 方法表与工具 chrome 过线之后，产品 TUI 已是 `HttpWsClient` attach，但可观察体验仍低于归档前的同进程 TUI。本票在 c2315（一条命令 loopback）之前，把 attach 路径修到与旧 InProcess 产品面同观察：MCP 头卡、steer/follow-up 条、切模型等操作不卡死、会话期内续联。

**全量对拍的边界**：兑现**已有** live specs 在 attach 上的 MUST（`app-tui-*` / `infra-mcp` / `protocol-app` / `server-core`），不是新开 Web、ACP、或把 PTY e2e 升成硬闸。已知 attach 仍红的已交付能力 MUST 在本票内变绿，禁止留给 c2315。

## Why

方案 C 能聊不等于能当产品 TUI。当前 attach 上至少四条已有合约被打破：

- 头卡 `mcp: N configured · 0 connected`：`loaded_resources` 读的是从未 connect 的 Host 资源壳，不是会话 writer 上的真连接态（违反 `pa-map3` / `sr-resource1` / `atc18` / `ath23`）。
- 忙碌二次提问：队列条先画上，随后被 Remote 空 `queue_stats` 抹掉；同文案注入还会被用户气泡去重吃掉（违反 `ati11` / `ati12`）。
- 切模型 / 许多写者操作「冻住」：unary 内 `wait_mcp_bootstrap` 堵住输入环；每个 `drain_pending` 还同步 `block_on` 打 `get_state`（违反 `mcp7` / `mcp8` / `ath1` / `ath23`）。
- 续联：`run()` 每次提问新建 WS，`AgentEnd` 拆掉；空闲无 mux；断线没有按 `last_seq` 再订阅（`pa-cs4` / `sr4` / `w6` 在产品 TUI 上未兑现）。

c2315 若现在 apply，会把残 TUI 封进「一条命令」。

## What Changes

- Host `loaded_resources` 返回进程内真实 MCP/skills 快照（连接中进度、已连接、诊断）；Remote 刷新头卡直到 settle；TUI 启动不因 MCP 阻塞第一帧。
- attach 上 steer / follow-up：队列条文案在入队后保持可见，直到 `QueueUpdate` 计数下降；禁止用空 `queue_stats` 覆盖本地条。需要登记 `queue_stats` unary 则本票扩表并双跑。
- 注入的用户气泡：与上一条 idle 提交同文时仍 MUST 作为新的 `UiEntry::User` 可见（或等价：用户能从 transcript 看出第二次提问已上行），MUST NOT 只靠去重把第二次提问从画面抹掉。
- 产品 host 输入环 MUST NOT 在 tick / `drain_pending` 上同步 RPC；写者 unary MUST NOT 死等 MCP settle。`/model` 打开与选定 MUST 在 MCP 未完成时仍可操作（选定可走 NextTurn）。
- 产品 TUI attach：mux 在会话期内常驻（不随单次 `AgentEnd` 拆掉）；断线 MUST 用 `last_seq` 再 `subscribe`；收到 `session/resync_required` MUST 按 `w6` 再订而不是只打一行错并清零后放弃。
- 对拍闸：同一观察（头卡 MCP 行、队列条、模型切换后 footer、二次提问气泡）InProcess 与 HttpWs 双跑；先红后绿。

## Capabilities

- `app-tui-host` / `app-tui-chrome` / `app-tui-input`（已有条款收口 attach）
- `server-core` / `protocol-app`（快照真值、订阅生命周期、可选 `queue_stats`）
- `infra-mcp`（启动不阻塞面；门闸不堵 tick）
- `test-qa-gate` 仅当需要把双 carrier 对拍接进既有闸

## Impact

产品 TUI attach 达到可日常使用：MCP 可见、插话/续跑可见、设置不冻界面、断线可续。c2315 增加对本票的 `depends_on`。print / embed / 面本地键位主题不改拓扑。

## 非目标

c2315 一条命令、Web UI、ACP、把 `avs2` PTY 升成 `just qa` 硬闸、新开 Event 变体、退回默认 InProcess、为未交付能力预留 shim。

## Open Questions

已拍板（探索确认）：

- **登记** `queue_stats` 只读 unary，双 carrier 过闸。
- **启动即** `mux` + `subscribe`（不要等第一次 prompt）。
- 同文二次提问：注入的用户气泡 MUST 可见（修跨 idle 气泡的去重）。
- 本票验证含 `just test-tui-e2e-pty`，**不**升成 `just qa` 硬闸。

用户补充：两次输入只显示一条用户对话（不是「同一轮 Thought 重复」）；spinner 有时冻住，像 Host 联系断开或 unary 堵住输入环——并进切片 3/4。
