# BDD OTel 并行干扰机制与前瞻修复

## 现象（实证）

- `cargo test --all-features --test bdd`（并行）：`test_otel_r1484_dual_identity` 偶发失败；
- 同一套件 `--test-threads=1`：950/950 全绿；单场景 isolated 复跑：绿。
- 与 c2841 无因果：c2841 未触碰观测/全局态；该 flake 在 c2841 之前即存在（本仓 infra-otel 场景与 app-tui-host 日志场景的既有并行耦合）。

## 机制（源码依据）

1. **SpanCollectScope 是「进程级替换 sink」**：`SpanCollectScope::enter()` 安装 once demux reporter，并把全局 `SPAN_SINK` 替换为自己的 buf；其 `_excl`（`SPAN_COLLECT_EXCL`）只串行 **同为取 scope 的测试**，不串行「未取 scope 但发射 span 的测试」。
2. **产品侧观测是进程全局态**：`set_provider_trace_active` / `set_observation_io_tier` / `set_obs_session` 都是进程级 slot。任何测试 activate 后，后续 span 都汇入当前活跃的 `SPAN_SINK`。
3. **干扰发射族**：`steps_app_tui_host.rs` 的「以临时目录请求即时文件日志」（`host_pump_bdd`）场景调用 `init_logging` + `set_provider_trace_active(true)` **且不取 scope**；与 otel 场景并行时，其 span（含工具/agent 回合）写入 otel 的 scope buf → `dual_identity` 的「root span 必须由我导出」断言见到外来 span 而失败。另全局 obs session 槽被并行测试改写也会污染会话身份断言。
4. 为何 flaky 而非必现：取决于并行调度窗口内是否有发射者 overlap 在 otel 场景存活期内。

## 修复方案（按前瞻性与维护性排序）

### 方案 P1（推荐，架构根治+可维护）：观测身份/激活按会话或场景显式持有
把进程全局 obs slot 的「激活 + 会话身份」收敛为**显式 per-session 所有权**（obs slot 本就有 r1483「仅写者路径更新」的 product 语义）。
- 产品面：`set_obs_session`/obs lane 改为由绑定 session 的 driver 持有（multi-session host 本就该 per-session，见 docs 库与多客户端）；
- 测试面：`SpanCollectScope` 之外再提供 `PerTestObs`（场景级 reporter 句柄 + 场景级 session slot），otel 场景与任何 enable 观测的测试都持有自己的上下文，跨度互不污染。
- 优点：消灭整类竞态（不只是本测试）；与产品多会话方向一致。
- 代价：触及 `obs` 模块（product 面）与 ai-bridge trace 的 slot API，中等风险，需独立 SDD。

### 方案 P2（近期可落地，低风险）：观测全局闸“持锁即独占”
把 process 全局「激活观测」建模为**独占锁**：activates 的测试（otel fixture + app-tui-host 日志场景等）一律先获取同一把全局锁（`ObservationGate`），未 activate 的测试不碰 span，天然互斥。
- 优点：不改产品代码，只动 test-infra；choke point 单一（观测激活 helper）。
- 缺点：枚举谁需要锁仍是「约定」（靠 review）；新增观测测试忘记持锁则复发。

### 方案 P3（可选加固）：scope 记录按会话 uuid 过滤
`SpanCollectScope::records()` 按 `langfuse.session.id`/`xylitol.session.id` = 本场景 SESSION_UUID 过滤外来 span —— 但全局 slot 被并发改写时本场景 span 也可能带错 id，故需与 P1/P2 配合（不能单独根除）。

### 方案 P4（进程隔离）：infra-otel + app-tui-host 观测场景拆独立测试目标
分到专有 serial 目标（如 `--test bdd-otel`），与其他测试物理隔离。
- 优点：绝对隔离、不改产品。
- 缺点：拆分测试目标需接线 CI 与 just qa；观测语义仍在多进程间共享全局态（若未来同一进程测则复发）。

## 建议

近期以 **P2**（观测激活集中 + 独占闸）+ **P3** 过滤兜底进 test-infra；中期评估 **P1**（per-session obs，产品多会话正确方向，独立 change）；**P4** 仅作最终兜底不优先。P1/P2 均需作为独立（非 c2842 内）的 test/infra change 立项。
