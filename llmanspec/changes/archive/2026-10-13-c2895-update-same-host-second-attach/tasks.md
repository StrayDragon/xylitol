# Tasks

## T1 server-core：根提交 CAS 守卫 + session_busy

- [x] `src/app/server/host.rs`：prompt 准入处 `compare_exchange` run_inflight；失败回稳定 `session_busy`（信封内错误，HTTP 200）；AgentEnd 路径清位不变
- [x] specs：`server-core` 新 requirement（并发根提交稳定拒绝，含先到端自拒；单 TUI TUI 层映射插话不回归）
- [x] BDD：A 回合中第二根提交 → `session_busy`；单端插话仍入队（既有场景回归）

## T2 server-core：观景态订阅 + inflight 标志 + 定向重放

- [x] subscribe 应答携 `inflight`：RAW JSON 注入（`subscribed_value` 在应答 Value 加字段；**不动 fbs**，typed `SubscribeResult` 维持客户端兼容解码，design「线协议增量」方案）
- [x] `bind_mux_to_session`：inflight 时新订阅者置 observe 态（不重放 tape、tape 直播过滤；非 tape 帧与 AgentEnd 照收）；空闲 → 完整订阅；重挂（同会话再 subscribe）按当时状态定态
- [x] journal 重放定向：仅发新绑定 mux 订阅者；`session/subscribed` 确认维持广播；in-process broadcast 道不变
- [x] specs：`server-core` 新 requirement（观景态订阅语义 + 重放定向）
- [x] BDD：inflight 中第二订阅 → 无 tape 重放/delta、AgentEnd 可见、subscribe 应答 inflight=true；空闲第二订阅照旧；重放只达新订阅者

## T3 app-tui：attach busy 提示与重挂

- [x] remote driver：暴露 attach inflight 状态（订阅应答读取）；重挂 = 既有 SwitchSession 重订阅路径
- [x] TUI 固定区：观景态提示条（进行中 → 已结束可重挂两态），词表对齐 `docs/architecture/TUI信息呈现与固定区词汇.md`；resume 动作接重挂
- [x] specs：`app-tui-host` 新 requirement（busy 提示、结束转态、重挂动作）；BDD/单测按既有 harness 可达面落位

## T4 layer-architecture r1514 改写

- [x] 删豁免句，改写为真实第二 attach 合约；既有场景核对更新

## T5 验证

- [x] `cargo test --all-features --test bdd` 全绿；`llman-sdd validate c2895-update-same-host-second-attach --strict` 绿
- [x] 双实例手测路径：B attach 于 A 回合中（冷观景+提示）、A 结束后 B 重挂（完整直播）
