# Tasks：会话操作命令化与驱动 trait 一致性门禁

> 依赖：无。needs_specs_change: false。
> seam：runtime/kernel 单测 + 全量 BDD 回归地板 + occupancy 测试组 + 新门禁脚本负向验证。

- [ ] T1: trait parity 门禁——新增 `scripts/check_driver_trait_parity.py`，解析两份 `XyDriver`，比对共享方法 `(name, is_async, receiver, params, ret)` 五元组，两侧独有方法走白名单；经 justfile 接线进 `just qa`。
- [ ] T2: parity 负向验证——临时把某共享方法的 async 性改坏，确认脚本 exit 1；恢复后确认 exit 0。
- [ ] T3: 字段搬移——`reload_cwd`、`session_profile`、`prepare_hub` 搬进 `SharedSubsystems`，driver 侧访问器改为读共享槽。
- [ ] T4: 命令形状——新增 `SwitchSession`/`NewSession`/`ForkSession`/`SetSessionName`/`SetSessionNameFor` 五个 `DriverCommand` 变体与 actor 侧分派。
- [ ] T5: script hook 重入审计——确认 `session_before_switch` 与 `session_shutdown_resume` 在 actor 任务内执行不重入；若重入则改为命令分派前后由 driver 发起。
- [ ] T6: session.rs 改造——四个会话操作改走 `handle.request()`，删除 inline 分支。
- [ ] T7: 回归——全量 workspace 测试 + BDD + occupancy 全绿；`just qa` 与 `just qa-e2e` 实跑。
- [ ] T8: 收尾——`llman-sdd validate --strict` 通过；`state_mut()` 计数对比回写。
