# Tasks

- [ ] 1. **host**（serve）：`HostState::slot` 创建会话槽时改为 `self.factory.loop_registry().map(|r| r.derive())`，使每个槽持有自 root 冻结的独立链；`materialize_writer_at` 的 `LoopScope::Session` 挂载点保持不变。测：`cargo test -p xylitol-serve`。
- [ ] 2. **tests**（serve）：补 Host 层两槽隔离测试——root 注册 `mw-a` 后创建两个槽，root 再注册 `mw-b`，断言两槽链只含 `mw-a`；在槽 A 派生 registry 上注册 `mw-c`，断言槽 B 与 root 均不可见。测：`cargo test -p xylitol-serve`。
- [ ] 3. **门禁**（repo）：`just qa` 全绿。测：`just qa`。
