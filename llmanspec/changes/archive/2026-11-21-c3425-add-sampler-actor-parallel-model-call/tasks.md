# Tasks：SamplerActor 与并行模型调用

> 依赖 change：c3420——假设其特性已合入基线。needs_specs_change: false。
> 落地记录（2026-11-21）：SamplerActor 为每会话采样记账与并行通道（agent/sampler.rs）——submit_stream 登记后 spawn 独立任务（child token + 重试梯 + 可选进程级 Semaphore 注入点），ledger 按发起序记录 InFlight/Completed/Failed/Cancelled，aggregate 按发起序汇聚（后完成不重排）；UpdateConfig 仅对新 Submit 生效。旁路登记：generate_stage_name_with_model 改走 submit_stream（StageName），turn 末端 overflow/auto compaction 边界以 register_external(CompactionSummary) 进同一 ledger（r3373 明示本条只约束记账语义）。react 主通道的重试梯保留内联（AutoRetryStart/End 事件与流惰性耦合，迁出将改变事件时序）；r3373 场景与 sampler 单测（并发时序 / 取消 / 外部登记 / 配置快照）全绿。
> seam：runtime 单测（Fake XyModel 并发时序断言，HoldModel 先例）+ react 既有场景回归。

- [x] T1: 前置自检（强制门）——ChatStateActor 在位；缺失 → STOP 并报告。
- [x] T2: SamplerActor 骨架——Submit / Cancel / UpdateConfig / IsActive / ActiveCount 协议 + biased JoinSet 主循环。[blocked-by: T1]
- [x] T3: 并行通道——turn 内多请求 fan-out、child token 树、汇聚 = 发起序（完成事件完成序交错）；结果汇聚语义按 c3410 design 定稿项收口。[blocked-by: T2]
- [x] T4: 重试梯迁入——r3021 分类 + retry.rs 迁入子任务；AutoRetryStart / End 事件语义回归（r3304）。[blocked-by: T3]
- [x] T5: 旁路登记——generate_session_name 与 compaction 摘要改走 Submit（StageNameConfig 语义保持）。[blocked-by: T2]
- [x] T6: 信号量预留——可选全局 Semaphore 注入点（默认不限）；UpdateConfig 仅对新 Submit 生效（turn 起始绑定原则）。[blocked-by: T2]
- [x] T7: 全量验证——nextest + just qa + barrier 工具批不受影响断言。[blocked-by: T3, T4, T5, T6]
