# Tasks：Actor 执行模型合约重写

> 依赖 change：无（本组首个）。needs_specs_change: true——specs 重写即本 change 的实现体。
> seam：全部复用既有 BDD harness（主机泵 / ScriptedDriver / occupancy 测试组 / rstest-bdd 步骤词表），不发明新 seam。新增场景暂无 step 定义时处于「待实现」状态（validate 非严格不红；--strict / --check 预期红，记录于 proposal Further Notes）。
> 红名单与合并顺序：finalize 须在 c3415 落地后或维护者明确接受红窗；squash 单笔合并时机由维护者通知。

- [ ] T1: 基线自检与改写清单核对——worktree 基于最新 main；validate --all 绿；对照 research/03 锚点产出六处 capability 的 req 改写清单（含 @req 编号与现行文本摘录），存 research/05-rewrite-checklist.md。
- [ ] T2: 绑定分支——checkout -b sdd/c3410-update-actor-execution-contract（base=main）+ change attach，确认 stage=full。[blocked-by: T1]
- [ ] T3: 重写 agent-runtime.feature——r3304 表述重述、r1059 队列 + 撤销规则与新场景、r1028 abort 语义、r1060 QueueUpdate items 载荷、r2920 / r2921 actor 重述与 handoff 门、多端验收场景①-⑥；既有场景名保持稳定（防绑定断裂）。[blocked-by: T2]
- [ ] T4: 重写 workspace-assembly（r2910 / r2914 执行权威 → 会话 actor 表述）与 layer-architecture（client / host 表述，如涉及）。[blocked-by: T2]
- [ ] T5: 改写 agent-session-store（严格式演进条款、LEGACY 删除）、infra-plugin（桥退役 + 首用批准 + 清单 mcp 声明）、protocol-app（revoke 方法、QueueUpdate 形状）。[blocked-by: T2]
- [ ] T6: 校验与红名单管理——validate --all；预期红名单（r1060 旧计数场景绑定等）记入 proposal Further Notes；确认无既有场景改名。[blocked-by: T3, T4, T5]
- [ ] T7: 跨 change 合并顺序备忘写入 proposal Further Notes。[blocked-by: T6]
