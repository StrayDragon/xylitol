# Tasks：ChatStateActor 与会话格式重定版

> 依赖 change：c3415——假设其特性已合入基线。needs_specs_change: false。
> 落地记录（2026-11-21）：T2 的 FlushAndAck 以现有写路径语义承接（append 单行原子写 + sync_all 落盘确认后返回 ack；并发 ack 顺序与持久唯一性由新增单测 appends_are_durable_and_ordered_ack_barriers 断言）；ChatStateActor 的 actor task 独占化与 c3432 驱动物理化同墙（AgentCapabilities 可变语句），写路径仍经 LogSessionStore 单一入口（append / create / fork / commit_compaction），旁路写（todo_checkpoint / workflow custom 条目）核实为同经 append_session_entry。T3 实测：commit_compaction 仅追加标记条目，不物理重写既有行（新增单测断言 commit 后旧字节为前缀）；未知行处理维持行级 skip + warn 上限。COMPAT_MESSAGE 保留——它是 r1104/s26 要求的旧布局拒载守卫（非迁移路径），仅删除死代码 enforce_legacy_session_version 与 export 的 legacy 接受分支。

> seam：agent-session-store BDD + store 单测（MemoryStore / 目录双后端）。

- [x] T1: 前置自检（强制门）——SessionActor 在位、c3410 格式条款已合入基线；缺失 → STOP 并报告。
- [x] T2: ChatStateActor 骨架——独占状态与 store 写路径；FlushAndAck 屏障；ack 时序单测。[blocked-by: T1]
- [x] T3: 首项验证——commit_compaction 未知行行为实测（c3410 遗留待验证项）；结论回写 design 与 specs（如需）。[blocked-by: T1]
- [x] T4: 新格式落地——SESSION_VERSION bump、contextInjected 条目、header 字段审计、LEGACY 迁移路径删除（enforce_legacy / COMPAT_MESSAGE）。[blocked-by: T3]
- [x] T5: 写路径切换——append / create_with_fork 经 ChatStateActor；旁路写（todo_checkpoint / workflow）改道验证。[blocked-by: T2, T4]
- [x] T6: resume / 加载 / 导出导入按新格式；repair_partial_tail 回归。[blocked-by: T5]
- [x] T7: 全量验证——nextest + just qa + agent-session-store BDD 对齐。[blocked-by: T6]
