---
depends_on: []
branch: sdd/c1205-update-app-tui-reload-ux
base_sha: f1611239f68c0200259abe6f67724ed4fa17ba77
checkpointed: false
---

# /reload 进行中交互体验

> **一句话**：idle `/reload` 进行中用状态条 `Reloading` + 输入软闸；协作取消（Esc）；失败/取消可见；结束后仍出汇总滚动提示。
> **当前排序**：#4（2026-08-10 自 delayed-changes 升格入 active）
> **状态**：方案已钉；规划壳齐；已 Branch binding（`sdd/c1205-update-app-tui-reload-ux`）。Specs landing 进行中。

## Why

c1120 已接线 `/reload`，但 `handle_reload` 在 effects 泵内同步 `await reload_runtime()`，host 不进 tick → UI 像卡住（无 spinner、输入像冻死）。重载（尤其含 MCP）可长达数秒～门闸墙钟，缺少进行中反馈与输入锁定策略。

## Purpose

只改 **idle `/reload` 进行中交互**；重载步骤语义仍以 c1120 / ath20 为准。

## 已钉方案（2026-08-10）

### 进行中反馈

- **状态条** lead：`spinner + Reloading`（固定短词；**不做**本波步骤轮换 A2）。
- 复用现有 Loader/braille；`status.md` 允许词表增补 `Reloading`。
- **独立** host 态（如 `reload_active`），**MUST NOT** 冒充 agent `run_active`（避免下轮预告 / busy-slash 表被误伤）。
- 进行中 **不**往滚动区刷进度墙；结束后仍尾随一条 `Reload:`（或取消/失败变体）滚动提示。

### 输入软闸

- 可打字、可 Ctrl+G 外编。
- **Enter 提交**（普通上行 / slash / bang）拒绝；壳层通告 body：`reloading — wait`（渲染层拼 `Error: ` → 可见 `Error: reloading — wait`）。
- 二次 `/reload` 同样拒（软闸），不重入。
- 结束后解除闸；**保留** editor 草稿（不因 reload 清空）。

### Esc → 协作取消（本波交付）

现网 `reload_runtime` **不能**直接 drop future：`reload.take()` 未放回、MCP 先卸旧 manager 再 connect → 取消会丢 reload 句柄 / 泄漏连接 / 工具表半开。

本波 **协作取消 + 一致收口**（非事务回滚）：

1. Host：`select!`（或等价）响应 Esc → 请求 cancel（对齐 bang abort 节奏）。
2. Driver/MCP：步骤间检查 cancel；MCP **旧 manager 保留到新装好再换**；任意出口 **MUST** put-back `reload` 态并保证工具表再次 freeze（或明确 builtins-only 定稿）。
3. 取消后：解除软闸与 `Reloading`；**ScrollNotice** 报告取消（含已完成步骤摘要）+ **壳层通告**提示取消/失败类结果。
4. Esc 优先级（对齐既有 busy）：有 overlay → 先关槽；否则取消 in-flight reload；再才是 idle 默认 Esc（清编辑等）。

不做「点按 `/reload` 前」全量快照回滚（skills/context 已逐步写入；成本高易错）。

### 超时 / 失败

- 墙钟与 startup 对齐：单 server `MCP_SERVER_CONNECT_TIMEOUT`；门闸 `MCP_FIRST_TURN_GATE_TIMEOUT`（code-first，本波不新旋钮）。
- 失败/超时：进 `Reload:` 滚动提示诊断；**并**壳层通告（与取消同档可见策略）。
- 验证：harness **可注入慢/失败 MCP**（程序化）+ 手测挂死路径；禁止用无限 spinner 掩盖永久挂起。

### 其它默认（本波一并钉）

| 点 | 钉法 |
|---|---|
| Ctrl+C | `reload_active` 无 overlay 时 **同 Esc 取消**（不退出）；收口 idle 后才走既有清输入/退出 |
| 退出（`/exit` 等） | 软闸下若经 Enter 提交则拒；不得因 reload 挂死进程（panic/信号路径仍须恢复终端） |
| Overlay | 可关；从槽提交若等价「上行/slash」则走同一软闸 |
| mcp pending cue | `reload_active` 期间 **不**占 status 右侧（避免与 Reloading 抢语义）；结束后按既有规则刷新 |
| 跨面 | **仅 TUI**；Web 未实现，本波不写 Web 合约 |

## What Changes

- TUI host：`reload_active` + 软闸 + Esc 取消臂 + status `Reloading`
- effects：`/reload` 改为可与 tick/输入交错的泵（勿再整段堵死 host）
- Driver/MCP：cancel-safe reload（换接顺序 + put-back + freeze 收口）
- design：`status.md` 增补 `Reloading`；必要时 playground 一槽静图（非本波阻塞）
- harness：进行中可见 status；软闸拒提交通告；取消/失败路径；历史/草稿不变性
- live specs（landing 时）：`app-tui-host`（主）；视需要 `app-tui-chrome` / `app-tui-input` / `app-tui-commands`

## Capabilities

- `app-tui-host`（modify）
- `app-tui-chrome`（modify，status / toast）
- 视 landing：`app-tui-input`、`app-tui-commands`

## Out of scope

- 改变 c1120 重载步骤集合/顺序语义（仅加进行中 UX + 取消安全）
- MCP 启动策略本身（c1200 / c1900 既有）
- 步骤短词轮换（`Reloading skills` → `MCP`…）— follow-up
- Web / 跨面实现
- 真事务快照回滚到 reload 前

## Ethics

- risk_level: low
- prohibited_actions: 用动画掩盖永久挂起；锁死无法退出且无说明；不安全 cancel 留下半开 MCP/未 freeze 工具表
- required_evidence: harness（含慢/失败/取消注入）+ 手测最短路径
- escalation_policy: cancel 收口语义若与实现冲突，先停 apply 再问

## Depends

- c1120（已归档）— 行为前置，非未归档 `depends_on` 边

## Notes

- 2026-07-16：自 `c1205-update-app-tui-reload-progress-ux` 泛化重命名。
- 2026-08-10：explore 钉案——A1 `Reloading`；软闸 B1；Esc 协作取消；拒提 `Error: reloading — wait`；失败/取消 = ScrollNotice + toast。
- 2026-08-10：补 `design.md` / `tasks.md`（含 Ctrl+C=取消、精确 toast/notice 文案、MCP snapshot 换接）；仍不 attach。
