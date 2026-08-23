---
depends_on: []
---

# Attach 客户端连接重连状态机

## Why

attach 是产品 TUI 默认路径，但面侧 HTTP+WS 客户端目前是裸的：无重连、无退避、
无代际失效——断线即败或静默。长会话中 Host 重启 / 网络抖动都会把用户打回手动重开。
生产级客户端连接层的通行打磨点（外部对照见 research 笔记）包括：
握手首帧校验、存活时长归零退避、generation 失效旧循环、事件攒批合帧、
宽限期后才提示断线（瞬时抖动不打扰用户）。

## What Changes

- 握手校验：WS 首帧必须为 `server_hello` 且版本可接受，否则视为连接失败进入重连判定。
- 重连循环：指数退避；单次连接存活 ≥ 阈值则退避归零（防抖动升级）；
  generation 计数使旧连接循环的迟到消息自动作废。
- UX 分级：初始连接与重连各给宽限期（如 5s / 1s），超时才上**壳层通告**
  （chrome toast，词表见 `docs/architecture/TUI信息面与chrome词汇.md`），不新增信息面概念。
- 下行事件攒批合帧：短窗口内多条事件合并一次 UI 投影，降低高频工具流时的重绘压力。
- 重连成功后走既有快照投影重建 transcript（冷恢复合约不变），不回放 journal 实况磁带。

## 非目标

- 不做跨重启会话自动续跑（Host 重启后的续跑策略是 host 侧独立议题）。
- 不引入多 endpoint 自动漂移换血（managed-service 式恢复不在本票）。

## Impact

- `src/app/core/host_client/http_ws.rs` 为主要落点（现为 181 行薄客户端）。
- TUI host 侧接通告与恢复路径；BDD 补「断线宽限 + 重连后快照重建」场景。

## Further Notes

- 一手对照（外部实现的重连循环、握手序/generation/缓冲摘录）：[research/reconnect-machine-notes.md](./research/reconnect-machine-notes.md)
