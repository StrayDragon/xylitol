---
depends_on: []
branch: sdd/c2460-add-command-idempotency
base_sha: 50095ac9a0b48d08e133dc07fd0805362ad18e7c
checkpointed: false
---

# Unary Command 幂等准入：调用方产 ID、首次获胜

## Why

四象限信封的 unary Command 走 HTTP POST，客户端在网络抖动/超时下的重试可能造成同一命令双发；
steer / follow-up 类命令双发会向 ReAct 队列注入重复条目，用户可见为重复插话。
wire 层已有调用方生成的 `rpcId`（信封级，必带），但 Host 只把它当关联键回显
（`call_id` 只覆盖 server-request/response 象限；`Command.id` 在 host 方法表被丢弃），
Host 侧没有按 ID 去重的准入语义。pre-0.0.1 期内定键最便宜，越晚越贵。

业界通行解法是「调用方产 ID + 服务端唯一性判定 + 冲突显式报错」：
把去重点放在最了解重试语义的一侧（client），服务端只需一个唯一性约束，
无需请求指纹 / 去重窗口等启发式（外部对照见 research 笔记）。

## What Changes

- **幂等键复用信封既有 `rpcId`**（2026-09-01 拍板；代码事实：`ClientRequest.rpc_id` 已存在且
  客户端每次调用新造 UUID、`Command.id` 在 host 方法表被丢弃——本票零 wire 形状变更，
  把 `rpcId` 语义从「纯关联」升级为「关联 + 幂等」）。
- Host 对同 session 槽的 unary 准入按 `rpcId` 去重：**首次准入获胜**，重复提交回放首次结果；
  首次处理中的重复等待后回放，不并行执行。
- 同 `rpcId` 但 method / payload 不同的提交返回稳定冲突错误（`idempotency_conflict`，HTTP 仍 200）。
- 客户端侧：HostClient 新增可携 `rpc_id` 的 unary 入口；重试时复用同一 `rpcId`
  （自动重试循环与退避不在本票，归 c2480 消费）。

## 非目标

- 不改 server-request/response 象限既有 `call_id` 关联语义；`Command.id` 保持 correlation 语义。
- 不做跨进程重启的持久化去重账本（进程内即可满足当前单机场景）。
- 不引入通用 at-least-once 投影机制；不实现客户端自动重试循环。

## Impact

- `src/app/server/host.rs`：方法表分发处的准入账本（查重 / 回放 / 冲突 / 有界淘汰）。
- `src/app/core/host_client/`：keyed unary 入口，既有调用点零改动。
- `llmanspec/specs/server-core/`：新增 `@req:sr-idem1` 规则 + 3 条 `@executable` 场景
  （重试回放 / 冲突报错 / 处理中等待）。

## 决策记录

- 2026-09-01（ff 深挖拍板）：键载体 = 信封 `rpcId` 复用（否决新增 `idempotencyKey` 字段——
  未来需要时加可选字段是加法便宜的变更）；BDD 合约并入 `server-core.feature`。
  详见 `design.md`。

## Further Notes

- 一手对照（外部实现的幂等准入契约摘录）：[research/command-idempotency-notes.md](./research/command-idempotency-notes.md)
