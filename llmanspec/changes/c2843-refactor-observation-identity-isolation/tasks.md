# Tasks

## T1 — turn 身份归因收紧（观察面只读）

- 审查 `bind_session` 之后 turn/iteration/compaction 起点的 obs 身份来源：确认均收敛到绑定时的 `ObsSessionContext` 快照（`AgentTurnSpan::start_with_session` 路径），进程槽仅在启动/测试兜底处读取；有现读则改为快照注入。
- 校验：`cargo test -p xylitol --lib agent::runtime` obs 相关单测绿（otel6/7/8/10/12 断言不变）。

## T2 — SpanCollectScope 按会话过滤

- `package/…/trace.rs::SpanCollectScope` 增 `records_for(session_id)`（按 `xylitol.session.id` / `langfuse.session.id` 过滤）与单测。
- OTel BDD fixture `records()` 迁移到 `records_for(SESSION_UUID)`；外来 span 不再进入断言。
- 校验：既有 otel BDD 场景绿。

## T3 — 并行隔离探针与全量门禁

- 新增（或复用现有）「多 enable-观测场景并发」探针，证明并行下归属过滤生效（无 `--test-threads=1` 逃生）。
- `specs:` check_command `cargo test --all-features --test bdd` 并行跑稳定全绿（移除 c2841/c2842 的 --no-check 逃生理由）。
- `llman-sdd validate c2843-… --strict` 绿；clippy 0 新警告。

## 范围外（Phase B，独立 change）

- ai-bridge 观测槽 per-session 所有权、移除 `set_obs_slot_writes` 折衷、多会话观测归属。
