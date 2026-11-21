# 调研底稿：grok-build actor 形态与 turn 消息流

来源：xai-org/grok-build 公开仓库源码（GitHub blob 页全文抓取，2026-11-20）。结论供 c3410 系列设计参照。

## 一、grok-build 是什么

xAI 开源的终端 coding agent，纯 Rust workspace（约 97 成员），actor 化零新框架：全部用 tokio 原语（mpsc / oneshot / select / JoinSet / CancellationToken）自建。仓库：https://github.com/xai-org/grok-build 。官方新闻页抓取 404，存在性由 GitHub API 元数据与搜索摘要佐证。

## 二、actor 边界

| actor | 边界 | 关键机制 |
|---|---|---|
| SessionActor | 每会话一个 | select 主循环分发命令 + 空闲 flush + 取消臂 |
| ChatStateActor | 独占对话状态与持久化 | 命令枚举约 50 变体，mutation / query（oneshot 回执）分段 |
| SamplerActor | 只串行化全局采样状态 | 每请求 tokio::spawn 独立 task 并发流式 |
| leader 进程 | 多客户端 attach | flock / socket / zombie 驱逐（产品决策，非 actor 本体） |
| 工具与插件 | 不是 actor | 普通模块留在 session 内 |

关键源码：xai-chat-state/src/actor/mod.rs、xai-grok-sampler/src/actor/mod.rs、xai-grok-shell/src/session/acp_session_impl/{run_loop,turn,sampler_turn,tool_calls}.rs。

## 三、turn 消息序列（实测）

1. 客户端 Prompt 命令（oneshot 回执句柄）进 SessionActor 信箱；忙碌时入 pending_inputs 排队。
2. SessionActor 起 turn 任务（spawn_local，与主循环同 LocalSet）。
3. push_user_message_and_ack 后发 FlushAndAck 落盘屏障（oneshot）。
4. 每 step：BuildConversationRequest（ChatStateActor oneshot）→ Sampler Submit（completion oneshot）→ 请求 task 逐 chunk 发事件，完整响应经 oneshot 回 turn。
5. chunk 不等 turn：drainer task 翻译后经会话事件轨（ReplayBuffer 去抖）即时出站。
6. 有 tool_calls：顺序 prepare（权限）后全部 dispatch future 进 FuturesUnordered 并发，结果完成序回传，串行做 hook 与回写。
7. 结束：TurnCompletionMsg → FinalizationGate pop 队首 respond_to → durable TurnCompleted → 晋级下一条排队 prompt。

## 四、监督 / 取消 / 背压

- CancellationToken 全线；actor 主循环 handle 全 drop 即自杀。
- JoinSet + biased select 先 join_next 清理再收新命令；每请求独立 token。
- 关停分阶段：hooks → feedback drain → FlushAndAck 带超时。
- 信箱全部 unbounded（已知名风险面）；限流在别层：请求内 429 重试梯、会话 RateLimitWaitBudget（仅 subagent）、进程级信号量。

## 五、可迁移 / 不照搬

可迁移：命令枚举 + oneshot handle 协议；竞态判据放串行化点（actor 处理时重查 turn_active）；JoinSet + biased select；每请求独立 token；窄查询变体；持久化 Ack 入 actor。
不照搬：每会话独占 OS 线程 + current_thread 运行时（单机单用户假设）；leader 常驻单例；unbounded 信箱（改有界 + 显式背压）。

## 六、对 xylitol 的结论

grok-build 证明 actor 化的正确单位是「模式约定 + crate 边界」而非框架；会话 actor + 状态 actor + 采样 actor 三件套与本仓会话槽 / XySessionStore / XyModel 端口一一对应。
