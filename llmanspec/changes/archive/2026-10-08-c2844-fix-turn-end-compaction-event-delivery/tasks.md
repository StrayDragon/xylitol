# Tasks

## T1 — combine_run_streams 保序冲刷

- [done] 抽 `combine_run_streams`（可测）：AgentEnd/流尾前清干 side/queue 已排队事件，保序放行。
- [done] 单测 2 条（End 先于 AgentEnd / 无终点清尾）。
- 校验：`cargo test -p xylitol --lib agent::runtime::react` 45 绿。

## T2 — 摘要 reasoning-only 非空

- [done] `generate_complete` 双累积；空文本非空推理 → 采用推理；皆空才 Err。
- [done] 单测 2 条（reasoning-only 成功、全空仍 error）。
- 校验：`cargo test -p xylitol --lib agent::compaction` 绿。

## T3 — spec + BDD

- [done] `domain-compaction` r1919（turn-end 事件送达顺序）/ r1920（摘要 reasoning-only 非空）+ 场景。
- [done] BDD：steps_c2844（真实 run 的 overflow 压缩 + Orchestrator 纯推理模型）+ 绑定两个场景。
- 校验：`cargo test --all-features --test bdd` 并行 955/955 绿；`llman-sdd validate c2844 --strict` 绿。

## T4 — 门禁与收口

- [done] lib 1618 绿；clippy 无新增；check scripts 绿。
- 收口：`llman-sdd change finalize c2844-fix-turn-end-compaction-event-delivery --into 当前分支`（不带 --no-check）+ push。
