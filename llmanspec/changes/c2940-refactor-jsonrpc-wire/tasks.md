# Tasks

## A. SDD 收口前置

- [ ] 1. 绑定分支 `sdd/c2940-refactor-jsonrpc-wire`（`llman-sdd change start`），确认 proposal / design / tasks 完整
- [ ] 2. 用 `llman-sdd spec next-req-id` 分配新 `@req`（不复用旧 id），改写 `protocol-app`、`server-core`、`workspace-assembly`、`app-tui-bridge`、`app-tui-host`、`app-tui-transcript` 的线协议措辞；删除 r1908 / r1909 / r1910 / r1940 对应规则；`bash/output` 改名
- [ ] 3. 在绑定分支提交 specs（`docs(sdd): ...`），`llman-sdd validate c2940-refactor-jsonrpc-wire --strict` 通过

## B. 协议 crate（xylitol-protocol）

- [ ] 4. 新增 `message.rs`：`RequestId`、`Request`、`Notification`、`Response`、`Outcome`、`ErrorObject`、`Message`、`RpcResult` / `RpcError`、`HostDescribeValue`、下行参数结构；`PROTOCOL_VERSION = 4`、`WIRE_FORMAT_JSONRPC`
- [ ] 5. 重写 `codec.rs`：`parse_text`、`parse_payload`（batch）、`decode_message`、`encode`；保留 `JSON_DEPTH_LIMIT` 与 `json_nesting_exceeds_limit`；错误码与业务码映射
- [ ] 6. `event.rs` 改名 `ToolExecutionStart` / `ToolExecutionEnd`；补齐 `AgentStart`、`ThinkingLevelChanged`、`AutoRetryStart`、`AutoRetryEnd`、`SessionInfoChanged`；增加 `Unknown`（`#[serde(other)]`）
- [ ] 7. `method.rs` 下行名改为 `bash/output` 等；`registry.rs` 删除 `Method` 相关依赖（如有）；`lib.rs` 更新再导出
- [ ] 8. 删除 `src/v3/`、`src/envelope.rs`；`Cargo.toml` 删除 `fory`，更新 description
- [ ] 9. 协议单元测试：batch / notification / 空 batch / 非法 id / 深度边界 / 错误码映射 / Event 往返与 Unknown

## C. 新 crate（xylitol-wire）

- [ ] 10. 新建 `crates/xylitol-wire`（依赖 protocol + plugin-sdk + serde_json + thiserror），加入根 `Cargo.toml` 的 workspace members 与 workspace dependencies
- [ ] 11. 实现 `event_to_wire` / `event_from_wire` 与 `WireEventError`，逐变体显式映射（23 项 + Unknown）
- [ ] 12. 单元测试：23 个变体表驱动往返；`AgentEnd` 本地语义；`Unknown`；`TodoUpdated` 逐字段

## D. 载体（xylitol-serve）

- [ ] 13. 新增 `wire.rs`（`CONTENT_TYPE = application/json`、`MAX_BODY_BYTES`、`Reply`、HTTP 状态映射）；删除 `mapping.rs`、`v3_mapping.rs`；`lib.rs` 更新导出，删除 `looks_like_fory_envelope`
- [ ] 14. 重写 `listen.rs`：`POST /rpc`（413 / 400 / 200）、WS text-only（binary 以 1003 关闭）、`healthz` 保持；`OccupiedHost` 改为 `dispatch_text`
- [ ] 15. 重写 `occupancy/envelope.rs`：`parse_payload` → 逐条 dispatch → `Reply`；下行 `downlink_notification`
- [ ] 16. 重写 `occupancy/rpc_module.rs`：`dispatch_request(host, id, method, params)`；未登记 `-32601`；array params `-32602`；reverse 方法；恢复并改写原 `#[cfg(any())]` 测试
- [ ] 17. `occupancy/ws.rs`：删除 `ServerFrame` / `ClientFrame`；`downlink_notification`；`ReverseRpcGateway` 与 journal 保持
- [ ] 18. `occupancy/host.rs`：`append_and_push` 经 `event_to_wire`；`push_resources`、`push_bash_output`（`bash/output`）、审批 / 问卷通知改为 `Notification`；`handle_unary` 的 `host.describe` 返回 `{protocol: 4, formats: [jsonrpc]}`；业务码 `unknown_method` 归一为 `unregistered_method`
- [ ] 19. `occupancy/idempotency.rs`、`engine.rs`、`registration.rs` 检查 fory / v3 残留
- [ ] 20. serve 单元测试与 `Cargo.toml`：删除 dev-dep `fory`，添加 `xylitol-wire`

## E. 客户端（xylitol-app-tui）

- [ ] 21. 新增 `host_client/json_rpc.rs`（替代 `envelope_client.rs`）：request 文本、downlink 解码、describe 校验（protocol 与 formats）
- [ ] 22. `host_client/http_ws.rs` 仅发 text 帧；pending 按 id 字符串；删除 `ENVELOPE_FORMAT_ENV`、`with_envelope`、`envelope_on`、`stable_rpc_id`
- [ ] 23. `host_client/mod.rs` / `in_process.rs` 更新为 `Notification`；`HostClientError` 保持（不含版本名）
- [ ] 24. `driver/remote.rs`：删除 `new_v3`，`XyRemoteDriver::new` 为唯一构造；改用 `xylitol_wire::event_from_wire`；`bridge/tests.rs` 更新
- [ ] 25. 删除 `v3_mapping.rs`、`wire_event.rs`、`envelope_client.rs`；`lib.rs` 清理；`Cargo.toml` 添加 `xylitol-wire`，description 去掉 envelope 版本词
- [ ] 26. 客户端单元测试：json_rpc 解码、describe 协商两条拒绝路径、text-only

## F. 其他 crate 与根包

- [ ] 27. `xylitol-kernel`：`command_exec.rs` 的 `SessionTreeKind` 转换改为本地 match，删除对 `xylitol_serve::v3_mapping` 的引用
- [ ] 28. `src/cli.rs`：`XyRemoteDriver::new_v3` → `XyRemoteDriver::new`
- [ ] 29. `xylitol-app-print/Cargo.toml` description 去掉 fory 字样；`xylitol-protocol` / `xylitol-serve` / `xylitol-app-tui` / `xylitol-kernel` / `xylitol-runtime` / `xylitol-plugin-sdk` 中残留的 fory 依赖与注释清理
- [ ] 30. 根 `Cargo.toml` 删除 workspace 与 dev-dependency 的 `fory`；`cargo tree -i fory` 为空；`Cargo.lock` 重新生成

## G. BDD 与 occupancy 测试

- [ ] 31. 删除 `tests/bdd/steps_envelope.rs`、`bindings_envelope.rs`、`dual_rail.rs`；`suite.rs` 去掉对应模块
- [ ] 32. 新增 `tests/bdd/bindings_jsonrpc.rs` 与 `steps_jsonrpc.rs`，覆盖 design.md 第 10 节与 prompt 6.1–6.5 场景（场景名与步骤文本逐字一致）
- [ ] 33. 更新 `steps_server.rs`、`steps_protocol.rs`、`steps_remote_resilience.rs`、`steps_hooks.rs`、`bindings_workspace_assembly.rs`、`bindings_server.rs`、`bindings_protocol_app.rs`、`steps_app_tui_bridge.rs`、`steps_c2844.rs` 中的 v3 / fory 引用
- [ ] 34. 更新 `tests/occupancy/assembly_fixture_tests.rs`、`tests/occupancy/in_process_host.rs`：`RpcMessage` → `Notification`，`session/bash_output` → `bash/output`
- [ ] 35. 删除 protocol 的 codec conformance / size baseline 测试（随 `v3/` 删除）

## H. 门禁、脚本与文档

- [ ] 36. 删除 `scripts/gen_envelope.py`；`justfile` 删除 `codegen-envelope` recipe
- [ ] 37. 新增 `scripts/check_no_version_names.py`（只读，`--check`），按 design.md 9.2 实现；`just qa` 经 `check_*.py` glob 接线
- [ ] 38. 更新 `scripts/check_crate_graph.py`：`PACKAGE_DIRS` 加 wire；`EXPECTED` 加 wire 依赖；serve / app-tui 期望加 wire；删除 fory 条目；`kernel → protocol` 合法
- [ ] 39. `rustfmt.toml`、`.uncomment.toml` 删除 fory / v3 例外注释
- [ ] 40. 文档：根 `AGENTS.md`（产品信封段、硬规则加版本命名禁止）、`src/AGENTS.md`（crate 图加 wire 节点与 kernel→protocol 边）、`crates/xylitol-app-tui/src/AGENTS.md` 第 35 行、`llmanspec/AGENTS.md`（版本化命名禁止）
- [ ] 41. `.claude/skills/xylitol-dev`：`references/deps-candidates.md`、`SKILL.md` 触发行、`references/qa-gates.md`、`references/agents-authoring.md` 同步

## I. 验证与收口

- [ ] 42. `just fmt`、`just lint`、`just test`、`just qa` 全部通过（PATH 含 `~/.bun/bin` 以跑 spec 门禁）
- [ ] 43. `llman-sdd validate --all --strict` 通过；`cargo tree -i fory` 为空
- [ ] 44. 字面清扫 `rg` 结果逐类记录；剩余命中仅限 design.md 9.3 所列非线协议内容
- [ ] 45. 提交（Conventional Commits）；`llman-sdd change finalize c2940-refactor-jsonrpc-wire` 产出 `archive(sdd): c2940-refactor-jsonrpc-wire`；不 push、不开 PR
