# Tasks

## T1 — r57 修订（propose 已落地）
- [x] `test-infra.feature` r57 收窄措辞 + MUST NOT 文本自证条款。

## T2 — steps_c2827 拆分（映射不变量：步骤文本零变化）
- [x] 文本探针（`la_load` 系）集中 `steps_layer_probe.rs` 并模块 doc 显式标注「结构探针」。
- [x] 真行为步骤按域拆出（image / process / todo / host 等按现状体量定夺），`bindings_c2827.rs` 按 spec 域对应拆分。
- [x] `check_bdd_steps` + BDD 全量对拍绿（零文本漂移）。

## T3 — 时序去脆弱化（各项独立可回退）
- [x] W3：`w_agent_layer_trust_deps` / `w_trust_single_source` 重复函数体合并。
- [x] W5：删 `w_c2841_prompt_processed` 冗余 800ms sleep（3s 读循环兜底）；复核 serve 后 30ms sleep。
- [x] W6：`steps_remote_resilience.rs` 5ms 壁钟硬断言改相对判据。
- [x] W7：c2842/c2841 会话 id 硬编码解耦（场景族共用命名常量）。

## T4 — r57 真实化
- [x] kill-tree / resources_watch / serve 启动 / remote resilience 等外部时序场景等待主体以 `with_test_timeout` 包裹；替换 steps_c2827 的 with_test_timeout 文本探针断言。
- [x] r57 场景步骤文本与强化后的步骤实现同步更新（propose 期保留旧文本以维持映射）。

## T5 — 门禁收口
- [x] fmt / clippy / `just qa` 绿 + `llman-sdd validate --strict` 0 + re-review 标记。
