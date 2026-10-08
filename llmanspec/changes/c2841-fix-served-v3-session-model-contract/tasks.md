# Tasks

全部在 change 分支（当前 PR 分支）上实施。收口（finalize）不列任务。

## T1 — v3 prompt 上行会话身份保真（实现已完成，收尾校验）

- [done] `src/app/core/host_client/wire_v3_client.rs` `build_request`：`prompt` 走 `Request::Raw` 原文过线，保真 `session_id`/`cwd`/`model_id`/`thinking_level`。
- [done] 回归单测：`request_family_shapes` 断言 RAW prompt 且 payload 全字段保留。
- [done] 校验：`cargo test -p xylitol --lib wire_v3_client` 全绿。

## T2 — Host 生效模型同步下行

- [done] 在 `src/app/server/host.rs` 增加会话写者「当前已解析模型」读取与下行 helper（`sync_model_downlink`，复用 `append_and_push` 管线，`Event::ModelSelect`）。
- [done] 在 `bind_mux_to_session` 完成 replay 后，若写者已装配且持有模型，向新订阅者下发一帧模型同步（覆盖 attach / 重连 / 会话切换）。
- [done] 客户端 `driver/remote.rs`：DownlinkCtx 带模型缓存；ModelSelect 事件 → `resolve_model_info`（已缓存可用模型表回查，首次 attach 竞态时拉取一次重试）→ `cached_model`；`current_model()` 收敛。
- [done] `app/tui/mod.rs`：`apply_idle_downlink` 收到 ModelSelect 后刷新固定区（徽标收敛）。
- [done] 验证：BDD `model-sync-on-bind`（live host + 订阅下行）绿；实机 tmux 验证全新 attach 徽标即显配置默认模型、run 正常。

## T3 — Spec 落地（server-core + cli-entry）

- [done] `llmanspec/specs/server-core/server-core.feature`：新增 `@req:r1914`（prompt 会话身份保真）与 `@req:r1915`（生效模型状态同步）规则 + 嵌套场景。
- [done] `llmanspec/specs/cli-entry/cli-entry.feature`：修订 `@req:r1387 unset-model-display`，区分「用户显式配置默认模型」（显示为当前模型）与「仅 provider env 映射的厂商默认」（仍 NOT-SET）；追加 `configured-default-shown` 场景。
- [done] 校验：`llman-sdd validate c2841-fix-served-v3-session-model-contract --strict` 绿（change + server-core + cli-entry）。

## T4 — BDD 绑定与全量门禁

- [done] 为三个新场景绑定步骤与 bindings（`tests/bdd/steps_server.rs` / `bindings_server.rs` / `steps_cli_surface.rs` / `bindings_cli_entry.rs`）：`prompt-runs-on-subscribed-session` / `model-sync-on-bind` / `configured-default-shown`。
- [done] `cargo test --all-features --test bdd`：949/950 绿（唯一失败 `otel_r1484_dual_identity` 隔离复跑绿 = 既有 flaky，与本次改动无关）。
- [done] 全量 clippy 无声；`just qa` 等价非变更门禁待最终跑。
