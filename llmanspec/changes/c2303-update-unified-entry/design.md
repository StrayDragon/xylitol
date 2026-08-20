# Design：CLI serve / attach

代码真值：产品 TUI 已默认 probe `http://127.0.0.1:18790`，未在听失败；握手在 `XyRemoteDriver`。本票只改 **CLI 词表与可覆盖 URL**。

## 1. `xylitol serve`（扁平，对齐 DSH；Pre-0.0.1 无 `server` 别名）

| 现在 | 本票 |
|---|---|
| `xylitol server run [--port]` | `xylitol serve [--host] [--port]` 直接听 |
| `xylitol server stop [--lock]` | `xylitol serve stop`（只打印 SIGTERM 指引） |
| `xylitol server install` | `xylitol serve install`（仍 stub） |

- `--host` 默认 `127.0.0.1`，可显式 `0.0.0.0`。
- `--port` 默认 **18790**；`0` 让 OS 分配，stderr 打印实际端口。
- 关 TUI **不**停 listener。
- MUST NOT 保留 `server` / `run` 兼容别名。

失败文案（`attach_fail_message`）改为提示 `xylitol serve`。

## 2. 产品 TUI `--attach`

挂在 `TuiSurfaceArgs`（`xylitol` / `xylitol tui` / `xylitol tui run` 同源）。

- `--attach <url>` 默认 `http://127.0.0.1:18790`。
- `--port <n>` 在 TUI 表面：与 `--attach` 互斥时 `--attach` 胜；仅 `--port` 则 `http://127.0.0.1:<n>`。
- 未在听：非零退出 + `attach_fail_message`。禁止 InProcess 回退。
- print 不接受 `--attach`。

## 3. 不在本票

自动拉起、引用计数、`--trusted-host`、PID 杀进程、Web Origin 白名单。
