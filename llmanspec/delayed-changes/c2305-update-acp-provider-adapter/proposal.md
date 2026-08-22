---
depends_on:
  - c2301-update-stable-wire-protocol
  - c2290-update-standalone-host
  - c2302-update-host-multi-session
---

# 产品位：ACP provider 外层适配器（后置）

xylitol host 作 ACP provider。适配器把 ACP JSON-RPC 译成 **c2290 方法表**（再进同一 dispatch），把下行 `session/event` 等译成 ACP `session/update`。ACP **不共用**产品信封，不进核心闭集。证据：`c2280/research/acp-interop.md`。

## Why

外部 ACP 客户端要驱动本 host。native 契约稳定后再做。

## What Changes

- 会话 / 消息 / 审批 / 取消与 native 近 1:1。
- 适配层自补：`last_seq` 续传（v1 无、v2 有）；steer / 队列（v1 无）；单写者 + 只读 attach（ACP 无此概念）。
- 起步 ACP v1 + WebSocket（ACP 自己的通道）。排在 c2290 + c2302 之后。

## 开放决策

- 有没有真实外部消费者。
- 要 steer / 强续传时是否等 v2。
- 是否暴露 fs / terminal / elicitation。

## 非目标

改 native 闭集。做 ACP client。核心地基完成前开工。
