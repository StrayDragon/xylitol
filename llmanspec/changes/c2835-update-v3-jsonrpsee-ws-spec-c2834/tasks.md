- [ ] 1.1 codec 终态形状：`jsonrpc_response` 逐字回显 id（收 `Value`），新增
      `jsonrpc_invalid_request`(-32600)；`jsonrpc_method_not_found` 形状不变。
      单测钉：数字 id 回显为数字、字符串 id 回显为字符串、-32600 形状。
- [ ] 1.2 `rpc_module` 去 jsonrpsee：手写 `dispatch_raw`（registry 方法表 ∪
      `approve_tool`/`answer_question`）+ task-local `CALL` 保持；4 MiB 体上限；
      未登记方法走 -32601。单测：describe 成功、未登记 -32601、非法信封 -32600、
      超限拒绝、幂等/租约 task-local 侧写仍在。
- [ ] 1.3 调用点收口：`http.rs` POST 与 `ws.rs`/binary 分支改用新签名；
      `polish_rpc_json` 的 -32601 补码与 `X-Writer-Token` / `writerToken` 逻辑不动。
- [ ] 1.4 `Cargo.toml` 删 `jsonrpsee`（optional 声明与 `server` feature 项），
      `Cargo.lock` 同步；`just qa` 绿。

- [ ] 2.1 `PROTOCOL_VERSION` 2→3；确认 `host.describe`、in_process、oapi 文案、
      attach 预检（`remote.rs` 硬等值）都随常量走；测试断言随常量更新。
- [ ] 2.2 四象限死变体清理：删 `RpcMessage::ServerHello` 与 `RpcMessage::ClientResponse`
      及其测试痕迹；`just lint-all` 无 dead-code 告警。

- [ ] 3.1 调试通道定位落进代码注释与 oapi 描述：binary = 产品上行，JSON text = 调试通道，
      两者 MUST 同 dispatch（无第二套方法表）。
- [ ] 3.2 租约跨连接窗口专项（D6 三条观测点，按时间序单测试）：
      ① 首次非只读 mint 令牌（first-wins）② 他连接带旧/无令牌写 → `writer_conflict`
      ③ 同一条 WS 内后续 unary 用连接本地租约（v3 binary 分支同语义）。

- [ ] 4.1 specs 终态改写并落地到绑定分支：server-core r1778/r1796/r1803/r1804/r1809、
      protocol-app r1696/r1701/r1709；迁移期条款 r1902/r1909 收口（design §3 映射表）。
- [ ] 4.2 逐场景审计：`steps_server` 的 JSON POST 场景保留（调试通道语义），
      `steps_wire_v3` 场景补齐「调试通道与产品路径同 dispatch」的可观察断言；
      `llman-sdd validate --specs` 与 `cargo test --lib --all-features tests::bdd::` 全绿。
