# Tasks

## T1 — 监听器：fory-only `/rpc` + 拆 OpenAPI + 留 healthz

- [x] `POST /rpc`：非 fory 魔数（含 JSON 文本）走非法信封，不再 `dispatch_raw` 文本分支。
- [x] `WS /rpc`：拒文本业务帧；只处理 binary + ping/pong/close。
- [x] 删除 `/openapi.json`、`/docs` 路由与 OpenAPI 模块。
- [x] `GET /healthz` 与就绪窗口（starting 503 / ready 200）保持同端口。
- [x] `Cargo.toml`：去掉 OpenAPI 依赖；salvo `default-features = false`，只开本监听器实际用到的 feature（含 websocket；测试 feature 仅 test 依赖）。
- [x] 校验：healthz 单测仍绿；原 OpenAPI 单测删除或改为 404；JSON POST 单测改为 400。

## T2 — [blocked-by: T1] 客户端与 describe

- [x] 产品 attach / `XyRemoteDriver` 只构造 v3 客户端；去掉「库默认 JSON、TUI 再打开」双路径。
- [x] `host.describe` formats 只含 `fory-v3`；客户端要 jsonrpc 或不识别格式 → 致命、不降级。
- [x] 测试与 lab 里 `HttpWsClient::new` 默认 JSON 的调用点改为 v3。
- [x] 校验：lib 描述/协商测与 remote driver 测绿。

## T3 — [blocked-by: T2] BDD 少删：对拍改 fory-only，JSON 场景改拒绝

- [x] 保留行为覆盖并改步骤：`product-path-event-equivalence`、`product-path-session-snapshot`、`session-tree-raw-carrier`、`session-tree-deep-restore` 只跑产品路径，断言领域结果（不再 zip JSON 孪生）。
- [x] `json-text-rpc-rejected`：Then 为 JSON `/rpc` 被拒。
- [x] `openapi-debug-doc`：Then 改为两路径均不提供调试文档。
- [x] `cutover-requires-parity-green`：改为产品默认 v3 且 JSON 被拒（可观察，不读源码）。
- [x] `steps_server` 产品场景（租约、幂等、审批、订阅、prompt…）改 v3 客户端；非法信封/batch 断言覆盖非 fory body。
- [x] protocol-app / layer-architecture / app-tui-bridge 绑定步骤随 GWT 文本更新。
- [x] 校验：`cargo test --all-features --test bdd` 相对本 change 分支绿。

## T4 — [blocked-by: T3] 文档、注释、skill

- [x] 更新 `docs/architecture/远程体验与线协议.md` 与 server/host_client 模块注释：去掉「调试通道」产品承诺。
- [x] 新增 `.agents/skills/xylitol-dev-candidates/SKILL.md`：salvo（明文 loopback Host / WS）、jsonrpsee（已退役）、axum（未在锁内、不为此引入）、Scalar/oapi（已拆）。写「何时再拿出来」；一次性迁移细节不进 skill。
- [x] 根 `AGENTS.md` Skills 表加一行指针。
- [x] 校验：`just doc-check`；skill 可被 description 检索到。

## T5 — [blocked-by: T4] 门禁

- [x] `just lint` 与相关 lib/bdd 测绿；`just qa` 在 change 分支上相对 merge-base 全绿（live-provider 从既有 skip 语义）。

不在本任务列 finalize。
