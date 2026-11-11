---
depends_on: []
title: "Refactor session storage to append-only log"
branch: sdd/c3233-refactor-session-log-storage
base_branch: main
base_sha: 09df89fe20bb94d8189d57996584340e108db598
---

# 会话存储重整为追加日志（breaking）

> 取代已删除的 c3230（2026-11-11 对齐：方案 B + 一步到位不兼容旧格式）。

## Why

全仓隐式状态机扫描中会话存储命中 10 项（S1-S10，见 research/implicit-state-machines.md），根因是把「压缩」实现成物理搬运，于是需要多文件手工事务、代际反推、六处布局判定、双 leaf 回退。成熟系统（WAL / git / Kafka / event sourcing）的共同骨架证明：带父链的追加日志做唯一真值 + 派生视图，事务需求被消灭而非解决。

2026-11-11 对齐决策：① 方案 B（日志优先）；② 轮转双触发（自适应大小 + 压缩点），turn 边界对齐；③ 不做自动重分片（显式 repack 留逃生口）；④ 维护单元 lazy 挂载（会话打开快检 + CLI）；⑤ breaking：旧格式（v7 多文件 / legacy 单文件）不做迁移，无法解析时提示开新 session。

## What Changes

- 新布局：log/NNNNNN.jsonl（只追加、行级 JSONL）+ manifest.json（纯缓存：写头 / leaf / 轮转策略 / 索引状态，可丢弃重建）+ index/（稀疏 id 到 offset 锚点，Kafka 式）。
- 唯一写路径：append 一行 + fsync；轮转 = 换新文件（自适应阈值初始 64MB、按中位 turn 字节调、范围 [16MB,256MB]，只在 turn 边界或压缩点切）。
- 压缩 = 追加一条 Compaction entry（语义标记，零字节搬运）；投影双语义：LLM 视图在压缩处截断，树视图全量可浏览；压缩点顺带轮转（热路径锁进当前文件）。
- 分页：turn / entry 粒度 page token（leaf + anchor + direction + unit + limit）；尾部倒扫覆盖常见路径，稀疏索引覆盖 travel 旧分支。
- 维护单元（lazy）：会话打开快检（尾部截断、.tmp 清理、缓存新鲜度）+ xylitol sessions maintain CLI（全量重建 / 孤儿清理 / 健康上报）；无常驻任务。
- 兼容门：旧布局检测 → 显式错误「格式不支持，请新开 session」；新格式内嵌 format: log-v1，后续演进走可选字段，不再 breaking。
- 写锁：flock / 平台信号量，锁先于一切写创建，永真（消灭「无 manifest 视为持锁」窗口）。
- 演进护栏：读写仅经 XySessionStore port；派生物一律可丢弃可重建；JSONL 导出/导入为永久交换格式。DB 升级触发条件四条（跨会话搜索产品化 / 缓存重建超 1s / 并发写需求 / 分析面）写进 design，触发前不引入 DB。

## Capabilities

- agent-session-store（数据契约重写，现存 36 requirement 大部分改写为日志语义）；关联 agent-llm-projection（投影缝不变）。

## Impact

- breaking：旧 session 不可读（未推广期，影响可控）；测试夹具与 BDD 数据需重建。
- 范围：plugins/xylitol-official-plugin-session（manager 重写为主）、crates/xylitol-plugin-sdk（manifest/index schema 词表）。
- 架构选型全景（15 项 ADR：原因 / 比较 / 解决的问题 / 缺陷 / 改进）见 research/architecture-decisions.md。

## 决策记录（2026-11-11，全部已决）

1. （已决 2026-11-11）fsync 粒度 = turn 边界批量（一个 turn 一批 + 收尾批）。
2. （已决 2026-11-11）稀疏索引锚点密度 = 64 entry / 锚（定值）；基准仅作记录，偏离 2 倍以上才重议。
