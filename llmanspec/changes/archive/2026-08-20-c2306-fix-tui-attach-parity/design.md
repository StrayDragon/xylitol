# Design：attach 对拍旧 TUI

实现顺序：**先红对拍 → 修 Host 快照与队列 → 解冻输入环 → mux 常驻/续联 → 双 carrier 全绿**。方案 A（c2315）不在本票。

## 测试边界（seam）

| 切片 | 公共边界 | 不算数 |
|---|---|---|
| MCP 头卡 | `loaded_resources` unary → `LoadedResourcesSnapshot` → 产品头卡/`/mcp` 缓存 | 面直接 reach `infra::mcp`；只断言配置条数 |
| 队列条 | 产品 `drain_pending` + `XyRemoteDriver`（或真 Host 槽）入队后 strip 文案；`QueueUpdate` 计数下降后 FIFO 消退 | 只测 in-memory `enqueue_steer_strip` 而不经 Remote `queue_stats` |
| 不卡死 | harness 在 MCP 慢连接 / 写者 unary 进行中仍能推进 tick 与键入；`/model` 打开不 await MCP settle | OS CPU%；真网关 MCP |
| 续联 | 真 Host：`subscribe(last_seq)` 跨 `AgentEnd` 仍推 `session/event`；断开后再订从 `last_seq+1` 重放；`resync_required` 后再订 | 单测里假 WS 不经 Host journal |
| 对拍闸 | 同一观察 InProcess vs HttpWs（c2304 双 carrier 表） | 把 PTY e2e 当 `just qa` 硬闸 |
| PTY 冒烟 | `just test-tui-e2e-pty`：attach 产品路径一轮对话 + `/model` 后仍可输入；本票验证必跑 | 默认 `just qa` 强制该用例 |

BDD：能挂 `server-core` / `app-tui-input` 已有 feature 的挂 GWT。产品 host 行为（strip、tick 不阻塞）以既有 TUI harness 为主，与 `ath5`/`avs1` 同缝。禁止另开脱离 `.feature` 的 CLI 协议子进程。本票收尾必跑 `just test-tui-e2e-pty`（`avs2` 仍 ignore，不进 `just qa` 硬闸）。

## 根因（代码事实）

```text
loaded_resources  → Host.resource_driver（enable_reload_state，从不 begin_mcp_bootstrap）
会话 MCP 真连接  → SessionSlot writer（materialize 时 wait_mcp_bootstrap）
队列条           → 本地 enqueue_steer_strip 之后 Remote.queue_stats() 恒 0 → sync_queue 删文案
卡死             → 写者 unary 内 wait_mcp_bootstrap；drain_pending 每次 sync_runtime_chrome → current_model block_on get_state
续联             → XyRemoteDriver::run 每次新建 mux，AgentEnd break 后丢 WS
```

`pa-map3` 已禁止 Remote 把 `loaded_resources` 降成空快照；现状是「有配置、无连接」的假快照，同样违约。

## 切片 1：MCP 快照 SSOT

`loaded_resources` MUST 反映 Host 进程当前 MCP 连接态（configured / connecting / connected / 诊断），与 writer 是否已 materialize 无关。

推荐：进程共享一份 MCP 资源所有者（reload 基线已经是进程级）；snapshot 读这份，而不是另造一个从不 connect 的 reader driver。Writer materialize **MUST NOT** 再 `wait_mcp_bootstrap` 堵住 unary；MCP 继续后台，门闸留在首次 generate（`mcp8`），面用轮询/事件刷新头卡。

Remote `poll_mcp_bootstrap` / 头卡刷新：TUI 在 connecting 时 MUST 继续 `refresh_loaded_resources`，直到 connected 摘要或失败诊断（`atc18`）。

## 切片 2：队列条与二次提问

入队后 strip 文案是 TUI 本地真值，Host 深度是对拍源。

- Remote `queue_stats` 禁止再返回恒空并 `set_queue_badge(0,0)`。
- 推荐登记 `queue_stats` 只读 unary（不占写者），`drain_pending` 用它校准计数，**只允许 trim 到 Host 深度，禁止在本地文案尚未被 QueueUpdate 确认前把列表裁成空**。
- 若暂不登记：`drain_pending` 在 Remote 上 MUST NOT 用 `queue_stats` 覆盖 strip；只信 `QueueUpdate` + 本地 enqueue。空闲无 mux 时条会与 Host 漂移——所以更推荐登记。
- `ati12`：steer/follow-up 注入的 `MessageStart(role=user)` 即使与上一条 User 正文相同，transcript MUST 仍可见该次提问（去重不得跨「已提交 idle 气泡」吞掉注入气泡）。

## 切片 3：输入环不阻塞

- `current_model` / `available_models` / `get_commands` 等同步 Driver 缝在 attach 上 MUST 走缓存；禁止每个 tick `block_on` HTTP。
- `SetModel` / 打开 `/model` 列表 MUST 不 `wait_mcp_bootstrap`。
- 首次 generate 的 Assembling 门闸保持 `mcp8`：跨 tick，不堵 host select。

## 切片 4：mux 常驻与续联

产品 attach 生命周期：

1. 启动：`host.describe` → `mux` → `subscribe(session, last_seq=0)`。
2. `prompt` / `steer` 等 unary 不再每次新建 WS。
3. `AgentEnd` **不**拆 mux；后续 `QueueUpdate`、MCP 无关的 session/event、下一轮 prompt 事件继续推。
4. WS 断开：指数退避再连 mux，unary `subscribe(last_seq)`；`resync_required` → 按 `w6` 再订（可 `last_seq=0` 全量），TUI 以 journal 重放重建 transcript，MUST NOT 只留一行错然后放弃。
5. 退出 TUI 才关 mux。Host 进程不随 TUI 退出（c2290 方案 C 不变）。

`XyRemoteDriver::run` 改为消费常驻下行，而不是「开 WS → prompt → AgentEnd → 关」。

## 切片 5：对拍闸

先写失败观察（头卡 0 connected、steer 后 strip 空、`get_state` 在 drain 路径被同步调用、AgentEnd 后 QueueUpdate 丢失），再接线到绿。

双 carrier：InProcess vs HttpWs 对 `loaded_resources`、`queue_stats`（若登记）、`subscribe` 跨回合、resync。产品 harness 覆盖 strip 文案与 tick 可键入。

## live specs（Branch binding 后）

- `app-tui-host`：attach 不在 drain/tick 同步 RPC；mux 常驻；MCP 头卡刷新直到 settle。
- `app-tui-chrome` / `app-tui-input`：头卡 connected 摘要；队列条不被空 stats 抹掉；同文二次提问可见。
- `server-core` / `protocol-app`：`loaded_resources` 真连接态；可选 `queue_stats`；subscribe 不随单次 AgentEnd 结束。
- `infra-mcp`：Host 写者 unary 不 `wait` 全部 MCP（与 mcp7/mcp8 对齐 attach）。
