# 设计：观测身份与激活的 per-session 所有权（Phase B）

## 现状实证（本轮调研）

- c2590 / otel24 已将 generate / turn / tool 归因切到**显式快照**；进程槽仅剩 name 与 optionless 回退消费。
- 槽写入点全在 materialized 路径：`AgentCapabilities::set_session`（每次 bind）、driver `switch_session`/`set_session_name`/`set_session_name_for`、bootstrap 恢复。reader 需要 `obs_slot_writes(false)` 防 stomp——布尔门是 hotfix。
- otel BDD 的 name 场景（otel7）直接 `set_obs_session_name` 写槽跑，靠「bind 清 name」达成 unnamed 断言——这正说明槽承担了本不该承担的职责。

## 目标不变量

1. **materialized 会话路径（turn/iteration/tool/compaction/generate）的观测归因 MUST 来自运行时所持会话事实，与进程槽无关**；跨会话并发运行，各 span 只带自己的 session_id/name/树边。
2. **进程槽 = 默认身份**：仅 optionless/unknown 请求（remote_count、hooks None 臂、无 options 的 `ProviderRequestTrace::start`）与测试兜底消费；只由**显式 writer 事件**（rename、会话恢复）更新，读者绑定/切换既不读也不写。
3. **无布尔门**：从调用点结构保证（reader 路径不触达 writer 事件），删除 `obs_slot_writes`。
4. **收集断言 per-owner**：`SpanCollectScope::records_for(session_id)` 按 `xylitol.session.id` 过滤；BDD 断言只针对本场景会话 → 并行 CI 稳定（与 1 同一不变式）。

## 实施布局

| 文件 | 改动 |
|---|---|
| `agent/capabilities/session_ops.rs` | `set_session` 去槽写；新增 `set_session_name_fact`；`obs_session_snapshot` 用自有 fact；删 `set_obs_slot_writes`/`obs_slot_writes` |
| `agent/capabilities/mod.rs` | 删 `obs_slot_writes` 字段/默认值；增 `session_name` 字段 |
| `agent/runtime/react/mod.rs` | 删 passthrough；`ReActConfig` 线程传 `obs_session_name`（来自 capabilities fact），turn 起点不再读槽 |
| `app/core/driver/in_process/session.rs` | `switch_session` 去 gate/去槽写；`set_session_name`/active 分支改 fact + 槽（writer 事件） |
| `app/core/bootstrap.rs` | 会话恢复：fact + 槽（writer 事件） |
| `app/server/host.rs` | `new_reader_driver` 删 `set_obs_slot_writes(false)` |
| `xylitol-ai-bridge/provider/trace.rs` | `SpanCollectScope::records_for(session_id)` + 单测 |
| `xylitol-ai-bridge/provider/obs_session.rs` | 文档收窄「默认身份」语义（API 不变） |
| `tests/bdd/steps_otel_obs.rs` | `records()` 走 `records_for(SESSION_UUID)`；otel7 改名走产品路径（`AgentRuntime::set_session_name`） |
| `app/core/driver/in_process/tests/session.rs` | otel25 测试改写为「switch 不写槽」结构性断言 |

## 明确不做（保持现状并记录）

- 激活闸（`provider_trace_active` / io tier）维持进程级**组合根策略**（served 拓扑下激活是服务端策略，本就该全会话共享）；测试内的 TLS `ObsGateScope` 已有；并行隔离靠内容过滤（#4）而非枚举闸者。
- `ProviderRequestTrace::start` / `merge_opencode_attribution` 的 None-arm / remote_count 继续消费默认身份（文档标注），不逐一改造成快照注入（超出本 change 安全面；optionless 在 c2590 后已近乎无生产触发）。

## 验收

- `cargo test --all-features --test bdd` **并行**稳定全绿（尤其 `dual-identity` 与 otel7），不再依赖 `--test-threads=1` 逃生，也无需 c2841/c2842 的 `--no-check`。
- 产品单测全绿；served 实机：attach + `/model` + run 归因正常；`just qa` 通过。
