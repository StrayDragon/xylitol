# Tasks：会话操作命令化与驱动 trait 一致性门禁

> 依赖：无。needs_specs_change: false。
> seam：runtime/kernel 单测 + 全量 BDD 回归地板 + occupancy 测试组 + 新门禁脚本负向验证。

- [x] T1: trait parity 门禁——新增 `scripts/check_driver_trait_parity.py`，解析两份 `XyDriver`，比对共享方法 `(name, is_async, receiver, params, ret)` 五元组，两侧独有方法走白名单；经 justfile 的 `qa-check-scripts` glob 接线进 `just qa`。
- [x] T2: parity 负向验证——临时把 `steer` 的 async 性改坏，脚本报 1 处失配并 exit 1；恢复后 exit 0。
- [x] T3: 字段搬移——`reload_cwd`、`session_profile`、`prepare_hub`、`prepare_acked`、`session_id`、三个 gateway 搬进 `SharedSubsystems` 的 `SharedRuntimeProbe`；driver 侧 setter 写穿共享槽。
- [x] T4: 命令形状——新增 `SwitchSession`/`NewSession`/`ForkSession`/`SetSessionName`/`SetSessionNameFor`/`BindSession` 六个 `DriverCommand` 变体与 actor 侧分派。
- [x] T5: script hook 重入审计——`session_before_switch` 与 `session_shutdown_resume` 在 actor 任务内执行；两者只走 runtime 的 `script_hook_cancel`/`script_hook_observe`，不回调 driver，未发现重入。
- [x] T6: session.rs 改造——`fork_session`/`switch_session`/`new_session`/`set_session_name`/`set_session_name_for` 五个方法改走 `handle.request()`，inline 分支删除。
- [x] T7: 回归——workspace 全量 4798/4798 通过；occupancy 48/48；MCP 21/21；`just qa` 61/61 exit 0；`just qa-e2e` PTY 32/32 exit 0。
- [x] T8: 收尾——`state_mut()` 从 46 处降到 35 处（`session.rs` 9→1，`prepare.rs` 6→5，`mod.rs` 31→29）；`DriverCommand` 变体从 6 个增到 11 个，构造点从 4 处增到 10 处。
