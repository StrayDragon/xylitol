# Tasks

全部在 change 分支（当前 PR 分支）上实施。收口（finalize）不列任务。

## T1 — v3 prompt 上行会话身份保真（实现已完成，收尾校验）

- [done] `src/app/core/host_client/wire_v3_client.rs` `build_request`：`prompt` 走 `Request::Raw` 原文过线，保真 `session_id`/`cwd`/`model_id`/`thinking_level`。
- [done] 回归单测：`request_family_shapes` 断言 RAW prompt 且 payload 全字段保留。
- 校验：`cargo test -p xylitol --lib wire_v3_client` 全绿。

## T2 — Host 生效模型同步下行

- 在 `src/app/server/host.rs` 增加会话写者「当前已解析模型」读取与下行 helper（复用 `append_and_push` 管线，`Event::ModelSelect`）。
- 在 `materialize_writer_at` 装配写者、`restore_model` 成功后（且会话已有订阅者时）广播一帧模型同步。
- 在 `bind_mux_to_session` 完成 replay 后，若写者已装配且持有模型，向新订阅者下发一帧模型同步（覆盖 attach / 重连 / 会话切换）。
- 单元测试：`src/app/server/host.rs` test 模块添加用例——装配带 `default_model_id` 的写者 + 订阅者绑定后，订阅者收到 ModelSelect 形状的 `session/event`。
- 校验：`cargo test -p xylitol --lib server::` 相关用例绿。

## T3 — Spec 落地（server-core + cli-entry）

- `llmanspec/specs/server-core/server-core.feature`：新增规则——v3/产品 RemoteDriver 的 prompt unary MUST 携带会话身份使 run 路由到已订阅会话；新增规则——Host 装配/绑定会话写者时 MUST 向订阅者同步生效模型（ModelSelect 下行），使徽标收敛。
- `llmanspec/specs/cli-entry/cli-entry.feature`：修订 `@req:r1387 unset-model-display`——区分「用户显式配置默认模型」（显示为当前模型）与「仅 provider env 映射的厂商默认」（仍 NOT-SET）；追加对应场景。
- 每条新/改规则 MUST 有嵌套 `场景:`（GWT），并在步骤层绑定。
- 校验：`llman-sdd validate c2841-fix-served-v3-session-model-contract --strict`。

## T4 — BDD 绑定与全量门禁

- 为 T3 新增场景在 `tests/bdd/steps_server.rs`（或合适 steps 文件）绑定可执行步骤。
- 校验：`cargo test --all-features --test bdd` 绿；`just qa` 或等价非变更门禁绿。
