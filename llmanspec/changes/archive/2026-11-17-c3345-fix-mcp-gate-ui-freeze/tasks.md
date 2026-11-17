# Tasks

- [x] t1 `serve`:`arm_tool_freeze` 未冻结时立即返回当前快照,后台任务以非阻塞 gate 短锁轮询至冻结并经 `session/resources` 回推权威快照(`crates/xylitol-serve/src/occupancy/host.rs`);同步更新 `tests/bdd/steps_server.rs` 绑定,使 server-core r1923 / r1925 新场景步骤文本映射(含「立即返回」与「下行收口」断言)。验证:`cargo nextest run --all-features --test bdd` 中 arm freeze 相关场景绿(在 change 分支上相对 merge-base 测)。
- [x] t2 BDD 绑定改写:`tests/bdd/bindings_server.rs` 场景更名 + `tests/bdd/steps_server.rs` then 步骤改写(「立即返回当前快照且冻结决议经下行收口」「不取写占用立即返回快照」),断言 arm 不等门限时且决议在门限时内收口。验证:`cargo nextest run --all-features --test bdd` arm freeze 两场景绿(在 change 分支上测)。
- [x] t3 app-tui harness 慢冻结切片:ScriptedDriver gate 可控延迟,断言 gate 窗口内主循环持续处理输入 / tick(不冻结)且冻结解除后 gated submit 自动上行(`crates/xylitol-app-tui/src/harness.rs`)。验证:`cargo nextest run -p xylitol-app-tui` 相关切片绿。
- [x] t4 回归与门禁:触及 crate 的 fmt / clippy / 测试(runtime、kernel、serve、app-tui、BDD 套件);`llman-sdd validate c3345-fix-mcp-gate-ui-freeze --strict` 全绿(含 tasks 全勾)。
