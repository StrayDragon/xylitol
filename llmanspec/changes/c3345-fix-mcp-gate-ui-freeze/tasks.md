# Tasks

- [ ] t1 `serve`:`arm_tool_freeze` 未冻结时立即返回当前快照,后台任务以非阻塞 gate 短锁轮询至冻结并经 `session/resources` 回推权威快照(`crates/xylitol-serve/src/occupancy/host.rs`);同步更新 `tests/bdd/steps_server.rs` 绑定,使 server-core r1923 / r1925 新场景步骤文本映射(含「立即返回」与「下行收口」断言)。验证:`cargo nextest run --all-features --test bdd` 中 arm freeze 相关场景绿(在 change 分支上相对 merge-base 测)。
- [ ] t2 `app-tui`:harness 慢冻结 gate 切片——ScriptedDriver 的 gate 可控延迟,断言 gate 窗口内主循环持续处理输入 / tick(不冻结)且冻结解除后 gated submit 自动上行(`crates/xylitol-app-tui/src/harness.rs`)。验证:`cargo nextest run -p xylitol-app-tui` 相关切片绿。
- [ ] t3 回归与门禁:触及 crate 的 fmt / clippy / 测试(`xylitol-serve`、`xylitol-app-tui`、BDD 套件);`llman-sdd validate c3345-fix-mcp-gate-ui-freeze --strict` 全绿(含 tasks 全勾)。
