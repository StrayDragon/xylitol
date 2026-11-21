# Design：ChatStateActor 与会话格式重定版

真源：c3410 design.md D5 / §4.3；本文只写本 change 特有形状。

## ChatStateActor

独占：AgentCapabilities 内会话状态 + LogSessionStore 写路径。写接口一律 FlushAndAck（追加落盘确认后返回）；读经 oneshot（或共享只读页缓存）。

## 格式 v(N+1)

- SESSION_VERSION += 1；等值门保留；LEGACY 迁移路径（parse.rs enforce_legacy / COMPAT_MESSAGE）删除。
- 新条目 contextInjected{ injector_id, point, placement, content_ref|preview, injected_at }，EntryBase 惯例沿用。
- 可选字段增删不升版（serde 默认忽略未知字段）——写入 store 模块 doc 备忘。
- 未知行：不引入 Unknown 变体；行级 skip 仅作为 repair_partial_tail 崩溃尾修复前哨。

## 迁移处理

旧版本文件等值门拒载，错误信息指向「不迁移」说明。首任务实测 commit_compaction 对未知行的行为（预期：无物理重写，仅标记条目），按结果定稿投影。

## seam

agent-session-store BDD 场景 + store 单测（MemoryStore / 目录双后端）。

## specs / BDD 绑定方案

- 无 live specs/** 编辑；agent-session-store 新规则（格式严格演进 / contextInjected 条目，见 c3410 §5.4）的 step 定义与 bindings 在本 change 落地（MemoryStore / 目录双后端 fixture：旧版本文件拒载场景、同版本完整解析场景、注入条目可读场景）。
- T3 实测结论若需 spec 修正（未知行处理细则），随本 change 直接编辑该 feature（既有条款改写走 WARNING 报告制）。
