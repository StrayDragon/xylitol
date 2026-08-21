---
depends_on: []
branch: sdd/c2340-add-attach-reload-cancel
base_sha: 637fc6eb3888bbae78c21d169b65cb056d9be41c
checkpointed: true
checkpoint_sha: f88f677f317b7132acc9ce5cc356e52f880e1b74
---

# attach 下 /reload 合作取消

产品 TUI attach 后，`/reload` 进行中按 Esc（`app.interrupt` / `app.clear`）只取消了本地 token——Remote `reload_runtime` 忽略该 token、阻塞等待 Host unary 返回，UI 卡在 reloading 直到 Host 自然完成。InProcess 路径同键位合作取消正常（C3 对拍缺口）。

## Why

- TUI 侧取消意图已就绪：reload 态下 interrupt/clear 键经 `request_reload_cancel()` 取消本地 token。
- Host 侧合作取消已实现：`abort` unary 命中进程级 reload 时调 `abort_reload()` 并回 `{cancelled:true}`——但该 wire 能力没有任何 client 使用。
- 缺口仅在 Remote `reload_runtime`：token 被忽略，取消语义在 attach 断链。

## What Changes

- Remote `reload_runtime` MUST 尊重注入的 cancel token：token 触发时向 Host 发 `abort` unary（复用既有合作取消），并以 `cancelled=true` 的报告收尾；不再等待原 unary。
- 产品行为对齐 InProcess：`/reload` 进行中取消 → reload 停止、UI 以已取消收尾、Host 不继续重装。
- 不改：Host abort→abort_reload 既有接线（本变更将其写入合约）、busy `/reload` 拒绝语义（ath20/ath23）、InProcess 行为。

## Capabilities

- `app-tui-host`：新增 `ath37`（attach reload 取消的端到端行为；harness 护栏，非 BDD）。
- `server-core`：新增 `sr-abort1`（abort unary 对进程级 reload 的合作取消语义，既有事实随本变更落约）。

## Impact

- 受影响代码：仅 Remote driver 的 reload_runtime 一处 + harness 护栏测试。
- 测试边界：`HostClient` trait mock 注入（remote 测试内 SnapClient 先例）；mock reload unary 挂起、abort unary 解挂并记录调用。
