# Tasks

测试边界：现有 `cli-entry` `.feature`（ce8/ce16/ce21）+ `cargo test --test bdd`。不新开脱离 `.feature` 的 CLI 子进程协议。握手不重测（c2302）。

## 1. 合约

- [x] 1.1 live specs：`cli-entry` ce8 / ce16 / ce21 — `serve` 取代 `server`；`--host/--port`；TUI `--attach`；失败文案提 `xylitol serve`
- [x] 1.2 `llman sdd validate c2303-update-unified-entry --strict --no-interactive --no-check`

## 2. CLI

- [x] 2.1 [blocked-by: 1.1] `CliCommand::Server` → `Serve`：扁平 `serve [--host] [--port]`；`serve stop` / `serve install`；删除 `server`/`run` 别名
- [x] 2.2 `TuiSurfaceArgs`：`--attach`、`--port`；probe 用解析后的 URL；`attach_fail_message` 提 `xylitol serve`
- [x] 2.3 `--port 0` 时 serve 打印实际端口；关 TUI 不停 Host

## 3. 闸

- [x] 3.1 `llman sdd validate c2303-update-unified-entry --strict --no-interactive`
- [x] 3.2 `just qa`（或 `cargo test --test bdd` + 仓库闸）
