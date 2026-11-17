# Design

`AutomationEngine::tick` 的 `load_leaf_branch` 错误按类型分流:

- `XySessionStoreError::NotFound { .. }` → 空分支(`Vec::new()`),继续正常 tick 逻辑(无 schedules → 无动作)。语义:未持久化 = 尚无任何自动化事实。
- 其余错误 → 原样 `AutomationEngineError::Store` 上抛(I/O、锁、序列化等仍是真错误)。

宿主 `automation_loop` 无需改动:NotFound 不再进入错误分支,WARN 消失;slot 驱逐时 task 由既有 `evict_slot` abort 收尾。

单测:`MemoryStore` 不创建会话(NotFound 形态)→ `tick` 返回 `Ok(vec![])` 且不产生动作;同会话 `append` 一条事实后 tick 正常读取(守护「只豁免 NotFound,不吞真错误」)。
