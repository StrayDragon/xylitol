# Tasks：ChatStateActor 与会话格式重定版

> 依赖 change：c3415——假设其特性已合入基线。needs_specs_change: false。
> seam：agent-session-store BDD + store 单测（MemoryStore / 目录双后端）。

- [ ] T1: 前置自检（强制门）——SessionActor 在位、c3410 格式条款已合入基线；缺失 → STOP 并报告。
- [ ] T2: ChatStateActor 骨架——独占状态与 store 写路径；FlushAndAck 屏障；ack 时序单测。[blocked-by: T1]
- [ ] T3: 首项验证——commit_compaction 未知行行为实测（c3410 遗留待验证项）；结论回写 design 与 specs（如需）。[blocked-by: T1]
- [ ] T4: 新格式落地——SESSION_VERSION bump、contextInjected 条目、header 字段审计、LEGACY 迁移路径删除（enforce_legacy / COMPAT_MESSAGE）。[blocked-by: T3]
- [ ] T5: 写路径切换——append / create_with_fork 经 ChatStateActor；旁路写（todo_checkpoint / workflow）改道验证。[blocked-by: T2, T4]
- [ ] T6: resume / 加载 / 导出导入按新格式；repair_partial_tail 回归。[blocked-by: T5]
- [ ] T7: 全量验证——nextest + just qa + agent-session-store BDD 对齐。[blocked-by: T6]
