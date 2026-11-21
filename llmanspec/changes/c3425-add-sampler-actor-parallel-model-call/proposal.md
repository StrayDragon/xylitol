---
depends_on: ["c3420-add-chat-state-actor-session-format"]
needs_specs_change: false
title: SamplerActor：模型调用并发化与并行 model call 解锁
---

## Why

关闭架构文件点名的「并行 model call 中间件表达不了」缺口（src/AGENTS.md:266），为 Plan-and-Execute / 评审回路 / 多模型协作提供执行基座。

## What Changes

- SamplerActor 串行化全局采样状态（注册表 / 取消 / 配置），每请求独立 task，JoinSet 与 biased select 先回收后收新。
- 并行 model call 通道：同 turn 多请求并发，结果汇聚语义在 design 定稿。
- 每请求独立 CancellationToken；429 分层（请求内重试 / 会话预算 / 进程级信号量）按 design 定稿。
- 按请求取消与请求级配置为预留能力，首个消费者场景：思考循环防护（跨 step 循环由 loop-policies 家族新中间件检测裁决，步内急停由按请求取消执行）。
- 旁路模型调用（标题生成 / compaction 摘要）统一登记进采样记账（react/mod.rs:101 的 generate_session_name 为现存旁路先例）。

## Capabilities

- agent-runtime（实现）
- runtime-model-registry（实现）

## Impact

- 现有 XyModel 端口与两个 provider 方言插件不动；并发入口收在 runtime 侧。
