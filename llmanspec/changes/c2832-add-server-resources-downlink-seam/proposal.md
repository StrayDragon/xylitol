---
depends_on: []
---

## Why

批 3 遗留：r1790（session/resources 下行推送）需要 server 真客户端循环缝（写者 poll → 快照变化才广播一帧 notification、不占 seq）。

## What Changes（设想）

- server 测试 harness 内挂 fake resources provider，驱动快照变化；客户端订阅捕获 notification。
- 断言：一帧、payload 与 loaded_resources unary 同形、不消耗 journal seq、旧客户端可忽略。
