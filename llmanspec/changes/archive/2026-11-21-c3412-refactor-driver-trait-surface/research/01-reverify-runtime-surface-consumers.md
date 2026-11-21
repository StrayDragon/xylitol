# T1 二次普查快照：runtime 侧无真实实现 / 消费者（2026-11-21）

> 实施门快照。基线：分支 sdd/c3412-refactor-driver-trait-surface（base main@522c922）。
> 结论：**剪贴板三件与 attach 六件在 runtime 侧均无真实实现、无消费者；未发现远端真实路径 → 不触发 STOP，按纯实现层重构继续。**

## 普查命令

```bash
rg -n 'copy_text_to_clipboard|stage_clipboard_image|read_clipboard_text' .   # 剪贴板三件
rg -n 'attach_session|link_health|attach_observe|take_resync_rebuild|drain_idle_events|refresh_surface_caches' .  # attach 六件
rg -n 'session_store' .
rg -n 'LinkHealth|AttachObserve|ClipboardCopyOutcome' .
```

## 剪贴板三件判定

| 位置 | 性质 |
|---|---|
| crates/xylitol-runtime/src/driver/proto.rs:251-270 | runtime trait 默认 unsupported 实现（待删的唯一 runtime 侧代码） |
| crates/xylitol-runtime/src/driver/types.rs:33 | ClipboardCopyOutcome 类型（runtime 侧消费者仅 trait 签名） |
| crates/xylitol-app-tui/src/driver/proto.rs:274-291 | app-tui 镜像 trait 默认（不动） |
| crates/xylitol-app-tui/src/driver/clipboard.rs、driver/remote.rs:1556-1568 | 真实实现（客户端本地 OSC52/native；不动） |
| crates/xylitol-app-tui/src/harness/scripted.rs:877-922 | ScriptedDriver 测试实现（app-tui 自有 harness；不动） |
| crates/xylitol-app-tui/src/effects/pending_ui/clipboard.rs:16,32、effects/slash/misc.rs:101 | 消费点，均经 app-tui 镜像 driver |
| tests/occupancy/assembly_fixture_tests.rs:264-272 | `use xylitol_app_tui::driver::XyDriver` + XyRemoteDriver → 经镜像，断言剪贴板不打 Host unary |

runtime 侧无 override、无消费者。XyInProcessDriver 无剪贴板实现。

## attach 六件判定

| 位置 | 性质 |
|---|---|
| crates/xylitol-runtime/src/driver/proto.rs:111-133 | runtime trait 默认 no-op（待删；LinkHealth/AttachObserve 类型随之成为孤儿） |
| crates/xylitol-app-tui/src/driver/proto.rs:126-152 | 镜像 trait 默认（不动） |
| crates/xylitol-app-tui/src/driver/remote.rs:1004-1098 | 真实实现（订阅 / 重放 / 链路相位；不动） |
| crates/xylitol-app-tui/src/lib.rs:142,228,377,391,546,553 | 消费点，均经镜像 |
| tests/bdd/steps_remote_resilience.rs、steps_server.rs:9（`use xylitol_app_tui::driver::{LinkHealth, XyDriver}`）、steps_app_tui_host.rs（`xylitol_app_tui::driver::AttachObserve`） | 全部经 app-tui 镜像 |
| tests/occupancy/assembly_fixture_tests.rs:585,734,1242-1416 | 经镜像 XyRemoteDriver |

runtime 侧无 override、无消费者。XyInProcessDriver 无 attach 实现。

## session_store() 现状核实（T4 输入）

- runtime trait：proto.rs:198（默认 None，待删）；in_process/mod.rs:728 override（待删）。
- react/mod.rs:142-143、593：`self.inner.session_store()` 走 AgentCapabilities 固有方法（session_ops.rs:145），**不经 driver trait**，无需改道。
- kernel bootstrap.rs:963：`agent.session_store()`，agent 为 AgentRuntime（固有方法），BootstrappedAgent.store 已直供 Arc<dyn XySessionStore>——kernel 直供形状已存在。
- kernel occupancy/factory.rs:247-248：trait 转发 override（待删）。
- app-tui 4 处（lib.rs:182、editor_history_seed.rs:99、host/editor_history.rs:109、effects/designing.rs:72）：走 app-tui 镜像 trait（attach 模式 None + 协议降级），镜像不动 → 4 处不动。
- BDD 绑定（steps_app_tui_bridge.rs:962-972「attach 模式不暴露会话存储」）走镜像 exposes_session_store，不受影响。

## 判定

未发现任何「runtime trait 侧真实实现或真实消费者」；不触发 proposal 逃生条款（无需转行为变更流程）。
