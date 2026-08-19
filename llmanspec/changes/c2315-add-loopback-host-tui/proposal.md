---
depends_on:
  - c2290-update-standalone-host
  - c2302-update-host-multi-session
  - c2303-update-unified-entry
  - c2304-add-conformance-gate
---

# 后置：方案 A（一条命令 loopback Host+TUI）

体感：一条 `xylitol` 同时起 Host 与 TUI。TUI 仍是 **真** `HttpWsClient`（POST + WS 下行），连本机 loopback，**不是** `InProcessDriver`。产品默认拓扑仍是 c2290 方案 C（先 Host 再 attach）；本票只加「一条命令」糖。

## Why

方案 C 要两个终端。有人要「开箱一条命令」又不想退回 TUI 直握 runtime。DSH 也有 web 进程内自服务的体感，但通道仍是 HTTP。MUST 等 c2304（方法表 + 工具 chrome + 会话/reload/MCP 对齐）归档后再 apply，避免一条命令仍是残 TUI。

## What Changes

- 一条产品入口：内部起监听器（或子进程 Host）+ TUI attach `http://127.0.0.1:<port>`。
- 关 TUI 是否停 Host、绑定已被占时是复用还是失败、Ctrl+C、孤儿 Host、同进程任务 vs 子进程：本票 propose 时必须三选一并写进 live specs，现在只记未决。

## 开放决策

- 绑定占用：失败 / 复用已在听 / 换临时端口。
- 生命周期：TUI 退出是否 SIGTERM Host；引用计数；孤儿回收。
- 同进程 tokio 任务 vs `xylitol serve` 子进程。

## Impact

恢复「一条命令即跑」体感，但协议路径与方案 C 相同。print / embed 不改。

## 非目标

改信封、换载体、符合性闸、Web UI、静默退回 InProcess。
