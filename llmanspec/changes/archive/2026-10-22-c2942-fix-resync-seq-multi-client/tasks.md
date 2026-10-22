# 任务：修复 resync 回放、transient 模型同步与多客户端订阅绑定

- [x] T1 协议层：`xylitol-protocol` 新增 `SESSION_MODEL`（进 `DOWNLINK_METHODS`）与 `SessionModelPayload`（`lib.rs` 再导出），补单测
- [x] T2 serve 绑定定向：`OccupiedHost::register_unbound_mux` 返回 `u64` 连接 id；`dispatch_text` 增 `mux_id: Option<u64>` 参数；`serve_mux` 只保留 id（不再持 `tx` clone，保证订阅者移除后 channel 关闭可被客户端感知）、POST 传 `None`；链路 `envelope::dispatch_text` → `dispatch_request` → `handle_unary` → `bind_mux_to_session` 打通；FakeHost 与 BDD 直调点更新签名
- [x] T3 bind 语义：`bind_mux_to_session(slot, last_seq, mux_id)`——`Some(id)` 按 id 从 unbound 取出并插入 subscribers（同 id 重订直接复用），`None` 保持 take-all；补 serve 单测（双连接定向、同连接重订回放、跨槽隔离）
- [x] T4 transient 模型同步：`sync_model_downlink` 改广播 `session/model`（无 seq、不写 journal）；补单测
- [x] T5 客户端投影：`app-tui` `json_rpc.rs` 增 `Downlink::SessionModel` 解码（含单测）；`remote.rs` 投影 `XyEvent::ModelSelect` 并更新缓存，保留旧 `session/event` 路径
- [x] T6 WS 入站错误 1009：`serve_mux` 入站 `Err` 置位标志并在收尾发 close 1009；不额外持 `tx` clone
- [x] T7 specs：`server-core`（r1803 方法清单、r1905 seq 语义、r1915/r1926 对齐 `session/model`、r2896 重订语义、r1804 超大帧收口）、`app-tui-host` r1265 重订措辞、`protocol-app` r1696 上行 notification 不执行不应答；`llman-sdd validate --specs` 过
- [x] T8 BDD：`steps_server.rs` 模型同步步骤改期望 `session/model`；`llman-sdd validate c2942-fix-resync-seq-multi-client --strict` 过
- [x] T9 门禁：`cargo fmt --all`、`just lint`、`cargo test -p xylitol-protocol`、`cargo test -p xylitol-serve --features listen`、`cargo test -p xylitol-app-tui`、`cargo test --all-features --test bdd`、`python3 scripts/check_no_version_names.py --check`
- [x] T10 提交：分支上按 Conventional Commits 分段提交（fix(serve)/fix(app-tui)/docs(sdd)），不 push、不 finalize
