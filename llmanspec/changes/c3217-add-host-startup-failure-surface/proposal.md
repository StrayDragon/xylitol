---
depends_on: []
title: "Surface host startup failure fast"
branch: main
base_branch: origin/main
base_sha: 78d077258664b34c5b9147336d76994cb6f448b7
---

## Why

产品 TUI 默认 ensure-host。若拉起的 Host 子进程在就绪前退出（例如配置里仍有已移除的 `transport: sse`），TUI 只会在 `READY_WAIT`（30s）后报就绪超时；这段时间没有任何提示，用户感知为“启动后卡住”。

## What Changes

- `spawn_detached_serve` 保留子进程句柄，并把 stderr 写入 0600 临时文件。
- `wait_until_ready` 在等待期间轮询子进程退出；一旦退出，立即返回可操作诊断：退出状态、stderr 摘要、日志路径。
- Host 未在听时，TUI 拉起前输出一行 `starting local Host at <url> ...`，避免静默等待。
- 成功路径不变：就绪后 detach 子进程并 attach。
- 单测覆盖“子进程提前退出”；现有 ensure-host 测试同步更新。

## Capabilities

- `user-experience`：ensure-host 诊断。
- `cli-entry`：TUI 启动提示。

## Impact

`xylitol-kernel` attach、根包 CLI TUI 路径。

## Further Notes

- 不改变 ready timeout；只把“子进程已退出”从超时中提前分离。
- stderr 捕获是 best-effort；读取后删除临时文件。
- 诊断仍指向 `~/.xylitol/logs/xylitol.log`，不替代日志。
