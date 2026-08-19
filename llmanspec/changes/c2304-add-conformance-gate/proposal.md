---
depends_on:
  - c2300-update-cs-capability-split
  - c2301-update-stable-wire-protocol
  - c2302-update-host-multi-session
---

# 产品位：符合性闸

同一张 Command → Event 场景表，对 embed 进程内实现与「测试 host + 真传输客户端」各跑一遍。未实现已承诺命令、或双实现漂移 → 红。真源 c2301。落地 `test-*`。

## Why

没有同表双跑，embed 与 attach 会再分叉。

## What Changes

- 场景覆盖 c2301 词面（含拒绝 / 缺 session / 写者冲突）。新协议语义必须带场景。
- 跨协议一侧走 c2302 真序列化与传输，不 mock 掉协议。
- 面本地不进表。
- 只读 / 写者冲突语义入表。

## 非目标

协议 / host / attach 本身。不是 E2E BDD。
