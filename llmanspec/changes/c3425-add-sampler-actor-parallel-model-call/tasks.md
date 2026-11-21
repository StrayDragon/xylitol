# Tasks：SamplerActor 与并行模型调用

> 依赖 change：c3420——假设其特性已合入基线。needs_specs_change: false。
> seam：runtime 单测（Fake XyModel 并发时序断言，HoldModel 先例）+ react 既有场景回归。

- [ ] T1: 前置自检（强制门）——ChatStateActor 在位；缺失 → STOP 并报告。
- [ ] T2: SamplerActor 骨架——Submit / Cancel / UpdateConfig / IsActive / ActiveCount 协议 + biased JoinSet 主循环。[blocked-by: T1]
- [ ] T3: 并行通道——turn 内多请求 fan-out、child token 树、汇聚 = 发起序（完成事件完成序交错）；结果汇聚语义按 c3410 design 定稿项收口。[blocked-by: T2]
- [ ] T4: 重试梯迁入——r3021 分类 + retry.rs 迁入子任务；AutoRetryStart / End 事件语义回归（r3304）。[blocked-by: T3]
- [ ] T5: 旁路登记——generate_session_name 与 compaction 摘要改走 Submit（StageNameConfig 语义保持）。[blocked-by: T2]
- [ ] T6: 信号量预留——可选全局 Semaphore 注入点（默认不限）；UpdateConfig 仅对新 Submit 生效（turn 起始绑定原则）。[blocked-by: T2]
- [ ] T7: 全量验证——nextest + just qa + barrier 工具批不受影响断言。[blocked-by: T3, T4, T5, T6]
