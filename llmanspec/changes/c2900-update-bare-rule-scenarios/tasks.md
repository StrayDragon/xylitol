# Tasks

## T1 protocol-app r1927 场景化

- [ ] `llmanspec/specs/protocol-app/protocol-app.feature`：r1927 规则块内补 `json-depth-limit-envelope-decode`（超限早退拒绝）与 `json-depth-limit-admits-tree-scale`（约两百层实机深树放行）两场景
- [ ] `tests/bdd/steps_protocol.rs`：补 `ProtocolBdd.last_decode` 字段与两对 when/then 步骤（`codec::decode_str` 断言）
- [ ] `tests/bdd/bindings_protocol_app.rs`：绑定两场景

## T2 server-core r1928 / r1923 / r1925 场景化

- [ ] `llmanspec/specs/server-core/server-core.feature`：r1928 补 `non-binary-body-is-illegal-envelope`；r1923 补 `arm-freeze-bounded-under-hanging-mcp`；r1925 补 `arm-freeze-lease-free-under-write-conflict`
- [ ] `tests/bdd/steps_server.rs`：`ServerTest` 补 `nonbinary_posts` / `freeze_*` 字段；补非产品二进制 body POST 步骤、arm_tool_freeze given/when/then 步骤（直连 `HostState::for_test_with_mcp` + `handle_unary`，复用 `tests/support/mcp_fixture_server.py` 先例）
- [ ] `tests/bdd/bindings_server.rs`：绑定三场景

## T3 验证

- [ ] `llman-sdd validate c2900-update-bare-rule-scenarios --strict` 绿
- [ ] `cargo test --all-features --test bdd` 全绿；`llman-sdd review` protocol-app / server-core unbound 归零
