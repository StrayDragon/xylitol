# design — c492 product `!` bash

## 代码事实（explore）

| 已有 | 缺口 |
|---|---|
| `Driver::execute_bash` / `Command::Bash` / `dispatch` → `DispatchOutcome::Bash` | 产品 host idle Enter 不识别 `!` |
| `XyBashResult { output, exit_code, … }` | 无产品 UI 渲染路径 |
| demo ati8：`!` → success 边框 | 产品 `UiRoot` 未 sync bash 边框 |
| demo Ctrl+G + `with_terminal_suspended` | 产品未接线 |
| c485 harness `ScriptedDriver::execute_bash` 返回 Err | 需可脚本成功路径测 scrollback |

## 硬决议

1. **前缀**
   - `!cmd` → bash，记入 context（`exclude_from_context=false`）
   - `!!cmd` → bash，排除 context（`exclude_from_context=true`）
   - trim 后匹配；去掉首个 `!`/`!!` 后的余下为 command（空 command → 系统提示，不执行）
2. **优先级（idle）**：slash `/` → bash `!`/`!!` → 普通 prompt。
3. **busy**：保持现有 steer / follow-up；**MUST NOT** 在 busy 时另开 `execute_bash`（避免与 abort/队列纠缠）。用户若 busy 提交 `!…`，按普通文本 steer（与今日一致）。
4. **Scrollback**：至少一行用户可见的 bash 调用摘要 + 输出（可截断）；非 0 exit → error/warning fg。不新增 Codex 浏览面；不强制新 `UiEntry` 变体（可用 `System` 或扩展 `Tool`——实现选最小者，design 不锁死类型名）。
5. **边框**：仅 idle editor 可见时随文本 live sync（每次输入后），色 = success；对齐 `design/bash-mode.md`。
6. **Ctrl+G**：产品最小 stub（系统行 + 可选回写）；真 `$EDITOR` 跟随 demo 环境变量策略，harness 不强制 spawn。

## 验收

| ID | Then |
|---|---|
| B1 | editor `!ls` → 边框 success |
| B2 | 去掉 `!` → 边框 muted |
| B3 | idle Enter `!echo hi` → `execute_bash` 被调且无 `run` |
| B4 | `!!…` → `exclude_from_context=true` |
| B5 | 成功输出出现在 scrollback |
| B6 | 非 0 exit 有错误色强调 |
| B7 | Ctrl+G harness：有调用记录 / stub 提示（不崩） |

## 非目标

c493、活树、改 `XyBashExecutor` 协议、产品 PTY 真 editor E2E。
