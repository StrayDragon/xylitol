# Tasks

测试边界：见 `design.md`。顺序 1 合约 → 2 MCP → 3 队列条 → 4 解冻 → 5 续联 → 6 闸（含 PTY 冒烟，不进 `just qa` 硬闸）。

## 1. 合约

- [x] 1.1 已 `change attach` 到 `sdd/c2306-fix-tui-attach-parity`（`base_sha` 钉本地 HEAD `977650e0`）；未走 `change start`（当时规划壳未提交）
- [x] 1.2 live specs：`app-tui-host`（drain/tick 禁止同步 RPC；启动即 mux+subscribe；MCP 头卡刷新到 settle）；`app-tui-chrome` / `app-tui-input`（头卡 connected；strip 不被空 stats 抹掉；同文二次提问可见）；`server-core` / `protocol-app`（`loaded_resources` 真连接态；`queue_stats` 只读 unary；subscribe 跨 AgentEnd）；`infra-mcp`（写者 unary 不 wait 全部 MCP）
- [x] 1.3 commit Specs landing；`llman sdd validate c2306-fix-tui-attach-parity --strict --no-check --no-interactive` 结构过闸（实现任务仍待 apply）

## 2. MCP 头卡（先红后绿）

- [ ] 2.1 [blocked-by: 1.3] 失败测：Host 配 2 个 MCP fixture、writer 已 connect 后 `loaded_resources` 仍报 `configured=2 connected=0`（当前 resource_driver 空壳）
- [ ] 2.2 进程级 MCP 资源与 `loaded_resources` 同 SSOT；snapshot 含 connecting / connected / 诊断；不另造从不 connect 的 reader
- [ ] 2.3 `materialize_writer` MUST NOT `wait_mcp_bootstrap`；MCP 后台进行；TUI attach 启动即刷新头卡直到 settle 或失败诊断
- [ ] 2.4 Remote `poll_mcp_bootstrap`（或等价刷新信号）在 connecting 时为真，避免头卡粘在 `0 connected`
- [ ] 2.5 双 carrier：`loaded_resources` InProcess vs HttpWs 快照字段一致（connected 非空或 diag 可见，禁止「仅 configured」假完成）

## 3. 队列条与二次提问

- [ ] 3.1 [blocked-by: 1.3] 失败测：busy 入队 steer 后 `drain_pending` 经 Remote `queue_stats` 把 strip 文案清成空
- [ ] 3.2 登记 `queue_stats` 只读 unary（不占写者）；`Command` + dispatch + specta；双 carrier
- [ ] 3.3 Remote `queue_stats` 走该 unary；`drain_pending` 只允许按 Host 深度 FIFO trim，禁止入队后立刻用 0/0 覆盖本地文案
- [ ] 3.4 失败测：两次相同用户正文（idle+steer 或两轮）transcript 只留一条 `UiEntry::User`
- [ ] 3.5 注入 `MessageStart(role=user)` 即使与上一条 User 同文也 MUST 另起气泡（或等价可见）；strip 随 QueueUpdate 消退后正文仍在 transcript

## 4. 输入环与 spinner 不冻

- [ ] 4.1 [blocked-by: 1.3] 失败测：MCP 慢连接时 `set_model` / 打开 `/model` / 首次写者 unary 堵住 harness tick（spinner 不再转、键入不进）
- [ ] 4.2 写者 unary 不 `wait_mcp_bootstrap`；`/model` 列表与选定不依赖 MCP settle
- [ ] 4.3 attach 上 `current_model` / `available_models` / `get_commands` 等同步缝走缓存；`drain_pending` / tick MUST NOT `block_on` HTTP
- [ ] 4.4 Assembling 门闸保持跨 tick（mcp8）；host select 在 Host unary 进行中仍能画 spinner、收键

## 5. mux 常驻与续联

- [ ] 5.1 [blocked-by: 1.3] 失败测：`run()` 在 `AgentEnd` 后丢 WS，随后 `QueueUpdate` / 下一轮 event 丢失
- [ ] 5.2 产品 attach 启动即 `host.describe` + mux + `subscribe`；`prompt` 不再每次新建 WS
- [ ] 5.3 `AgentEnd` 不拆 mux；会话退出才关
- [ ] 5.4 WS 断开：再连 mux + `subscribe(last_seq)`；`resync_required` 按 w6 再订并重建 transcript，MUST NOT 只打一行错后放弃
- [ ] 5.5 双 carrier：subscribe 跨回合仍收到新 `session/event`；断线重放从 `last_seq+1`

## 6. 闸

- [ ] 6.1 `llman sdd validate c2306-fix-tui-attach-parity --strict --no-interactive`
- [ ] 6.2 相关 TUI harness + server-core / app-tui-input BDD；`just qa`
- [ ] 6.3 `just test-tui-e2e-pty`：attach 产品路径一轮对话可见用户气泡与回复；`/model` 后仍可输入；不进 `just qa` 硬闸
