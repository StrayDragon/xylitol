# src/app/append_only/

独立 **append-only surface**（旁路观察 / sub-agent 轨迹）。与主会话 AO TUI 并列，不抢 TTY 默认。

## 角色

| 角色 | 职责 |
|---|---|
| 入口 | `SurfaceMode::AppendOnly` + CLI `append-only`；`run` 经 `XyDriver` |
| bridge / model | `XyEvent` → 已提交块；块帽 3/5 + session spill |
| keys | 闭集：Busy Esc abort、`/exit`、空闲双 Ctrl+C；无 fold/slash/plate |
| 载体 | 库 `InteractionMode::Inline`（实现细节；产品不教「Inline 模式」） |

## 硬约束

- MUST 复用 `composition::build_agent` + `XyDriver` / `XyEvent`；禁止 fork ReAct。
- MUST NOT reach `infra::*` / `agent::capabilities`（组合根 CLI 可设 spill root）。
- MUST NOT 注册主线 fold / Ctrl+O / Command Plate / 完整 slash 发现。
- 块帽：tool/bash/assistant **3**；write/diff **5**；不可展开；溢出 → session 旁 spill + `[Full output: …]`。
- ath30 主语 = 主会话 AO；本面是额外入口，不是 AO-only 削弱例外。

## 验证

单元 + ScriptedDriver harness（`cfg(test)`）。编排 M1 另票。
