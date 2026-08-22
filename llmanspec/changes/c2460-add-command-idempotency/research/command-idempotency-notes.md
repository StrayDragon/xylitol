# 命令幂等准入 外部对照笔记（c2460）

> 调研来源：外部参考实现（生产级 coding agent），2026-08-23 摘录。
> 本笔记仅作选型对照；关键结论已摘要进 proposal。

## 参考实现一手证据

**契约**（其 session 领域 spec 第 9 行附近）：

> Reusing a user or synthetic inbox item ID is idempotent when Session and type match:
> the first admission wins…

- prompt 命令 payload 支持调用方生成的可选 `id`；
- 同 ID + 同 session + 同类型 → 幂等（首次准入获胜）；不匹配 → ConflictError → HTTP 409
  （协议端点声明 `declaredStatuses: [409, …]`）。

**准入路径**：`Session.prompt()` 只做持久准入——发 durable 入队事件，
投影事务内插 inbox 行；ID 唯一性由存储层约束保证，重试天然安全。
客户端可自带 message ID 是「幂等重试」的前提（服务端不产准入 ID）。

## 设计动机

- 网络层 at-least-once 是常态（超时重试、代理重放），命令面必须自己回答「重复提交怎么办」；
- 「调用方产 ID + 首次获胜」把去重点放在最了解重试语义的一侧（client），
  服务端只需一个唯一性判定，无需请求指纹/去重窗口等启发式。

## xylitol 现状核对（2026-08-23）

- `src/protocol/wire/transport.rs:29`：`request_id: Option<String>` 存在但只是透传字段，
  无准入语义；`envelope.rs:138` 的 `call_id` 仅覆盖 server-request/response 象限关联。
- `src/app/server/host.rs` 方法表分发处无按 ID 去重；同 session 的 steer/follow-up
  双发会真实入队两次（ReAct 队列在 Driver，不在 wire 层）。
- 未发布产品，wire 形状可直接改（无兼容包袱），现在改最便宜。

## 落点判断

- Command 载荷加可选调用方 ID → host.rs 分发处对同 session 做唯一性判定：
  首次准入返回首次结果；冲突（session/type 不匹配）回冲突类错误。
- 进程内去重即可满足单机场景；跨重启持久账本明确不做（proposal 非目标）。
- BDD 补「同一命令带同 ID 重试不双发」可执行场景。
