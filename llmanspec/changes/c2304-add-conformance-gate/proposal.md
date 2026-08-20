---
depends_on:
  - c2300-update-cs-capability-split
  - c2301-update-stable-wire-protocol
  - c2290-update-standalone-host
  - c2302-update-host-multi-session
---

# 产品位：符合性闸

同一张 **c2290 方法表**（c2301 闭集作 payload；四象限作信封），对 `InProcessClient` 与 `HttpWsClient`（测试 host + 真 POST/WS）各跑一遍。未实现已承诺方法、或双 carrier 漂移 → 红。落地 `test-*`。

## Why

没有同表双跑，embed 与 attach 会再分叉。

## What Changes

- 场景覆盖 c2290 方法表（含拒绝 / 缺 session / 写者冲突 / 未知方法）。新协议语义必须带场景。
- 一侧 `InProcessClient`，一侧 `HttpWsClient` + c2302 真序列化与传输（POST + WS 下行），不 mock 掉协议。c2290 的 `InProcessClient` 现为 EchoHost 座位，**不是**真 dispatch；本票 MUST 换成与 Host 同表的 in-process 实现，禁止 echo 过闸。接上真 HostState 时 MUST 与 `HttpWsClient` 一样持有 `writerToken`（Echo 无租约）。
- 面本地（`Quit`、键、画）不进表。
- 只读 / 写者冲突语义入表。

## 非目标

协议 / host / attach 本身。不是 E2E BDD。
