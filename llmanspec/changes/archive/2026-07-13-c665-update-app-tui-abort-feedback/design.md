# Design — c665 abort UI + Esc mid-bang

## 反馈形态

| 时机 | UI |
|---|---|
| Esc abort（agent 或 bang） | scrollback **一行** `System: Aborted`；status **立即 idle（0 行）** |
| 随后流 `Error("aborted")` | 若已有 `Aborted` 注记则 **MUST NOT** 再插一条 |
| bang 运行中 | status `Running`（单行 spinner 族）；完成后 idle |

对齐 Compacting/Retry：短词 + 一行说明，不堆墙。

## Esc mid-bang

根因：`drain_pending` await `Command::Bash` 独占 `&mut Driver`，键盘不进 loop。

决策：

1. `Driver::execute_bash` → **`&self`**（InProcess 已只读 agent；Remote 用既有 `&self` HTTP）。
2. bang **不**在 `drain_pending` 内 await；`run_host_loop` 在 `select!` 中并发 `execute_bash` 与键盘。
3. bang 期间 `bash_active` ⇒ `is_busy()`，Esc → `pending_abort` → `driver.abort()`（c660 杀树）。

## 测试

- Harness：busy Esc → entries 含 `Aborted`、status idle。
- Harness：ScriptedDriver 可挂起 bash；Esc → `abort_count≥1` 且结果 cancelled。
