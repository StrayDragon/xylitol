---
depends_on: []
title: 宽限到期驱逐与订阅绑定互斥（修孤儿槽与写锁误释放）
branch: sdd/c3340-fix-linger-evict-subscribe-race
base_branch: main
base_sha: 463e01a2182fbfa0b5acf2c1a059f6b69a5ca34e
---

## Why

审计中风险 M1：宽限定时器醒来后只查回合标志与订阅计数，然后无条件 release_write_lock + evict_slot，不与订阅绑定互斥。与新订阅 bind 交错可产生孤儿槽（客户端假订阅）或把重连方刚取得的写锁提前释放（跨进程双写窗口）。server-core spec 已钉「宽限内同一进程重连 须续占」「释放前 须确认无订阅者」，竞态是对现行合约的违反。

## What Changes

- SessionSlot 增加 eviction_gate 互斥：宽限到期驱逐、干净断开驱逐、订阅绑定关键区三者串行
- 驱逐方在门内重检回合标志与订阅计数；订阅方在门内重解析槽（已被驱逐则取续接水位的新槽），孤儿槽不可能被绑定
- server-core spec「会话槽释放」规则补互斥与新槽续接语句
- host 单测：订阅先绑则驱逐放弃；驱逐先提交则迟到订阅取得新槽（确定性，tokio paused time）

## Impact

- 正常路径零行为变化（互斥仅串行化本就罕见的交错）；竞态窗口消除
