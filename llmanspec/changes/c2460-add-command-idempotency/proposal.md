---
depends_on: []
---

# Unary Command 幂等准入：调用方产 ID、首次获胜

## Why

四象限信封的 unary Command 走 HTTP POST，客户端在网络抖动/超时下的重试可能造成同一命令双发；
steer / follow-up 类命令双发会向 ReAct 队列注入重复条目，用户可见为重复插话。
当前 wire 层无可幂等键（`call_id` 只覆盖 server-request/response 象限，`request_id` 为可选透传），
Host 侧也无按 ID 去重的准入语义。pre-0.0.1 期内改 wire 最便宜，越晚越贵。

业界通行解法是「调用方产 ID + 服务端唯一性判定 + 冲突显式报错」：
把去重点放在最了解重试语义的一侧（client），服务端只需一个唯一性约束，
无需请求指纹 / 去重窗口等启发式（外部对照见 research 笔记）。

## What Changes

- Command 载荷/信封支持**调用方生成的请求 ID**（可选字段；缺省时行为不变）。
- Host 对同 session 的命令准入按该 ID 去重：**首次准入获胜**，重复提交返回首次结果或幂等成功。
- Session / 类型不匹配的同 ID 提交返回冲突类错误（409 类语义，见 research 对照笔记）。
- 客户端侧（面信封客户端）生成唯一 ID 并在重试时复用。

## 非目标

- 不改 server-request/response 象限既有 `call_id` 关联语义。
- 不做跨进程重启的持久化去重账本（进程内即可满足当前单机场景）。
- 不引入通用 at-least-once 投影机制。

## Impact

- `src/protocol/wire/`：Command 载荷加可选 ID 字段（serde 可忽略未知旧字段不成立——本仓未发布，
  直接改形状）。
- `src/app/server/host.rs`：方法表分发处的准入去重点。
- BDD 补一条「重试不双发」可执行场景。

## Further Notes

- 一手对照（外部实现的幂等准入契约摘录）：[research/command-idempotency-notes.md](./research/command-idempotency-notes.md)
