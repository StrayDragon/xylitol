# tasks — c3365-add-abort-turn-end-stamp

测试 seam（全部复用既有 harness，不新建）：

- runtime：`crates/xylitol-runtime` react 循环既有单测 seam（合成模型流 + cancel）。
- 产品端：`crates/xylitol-app-tui` session_tree / activity_fold 既有单测与场景构建器（SceneBuilder）seam；
  app-tui-transcript BDD（`tests/bdd` transcript steps）。

## T1 runtime：abort 轮次结束墙钟落盘

- [x] T1.1 react 循环跟踪「本轮已落盘 assistant」状态；所有 abort 终止路径在无 assistant 落盘时，
  落盘空 content / stop_reason=aborted / 无 errorMessage 的 assistant 标记并纳入 AgentEnd messages；单测覆盖流中取消与流前取消两路径，且断言已有部分 assistant 的轮次不追加标记。
- [x] T1.2 断言标记不进 project_for_llm（沿用既有 aborted skip 的回归单测）。

## T2 产品端投影：标记不可见但可分段

- [x] T2.1 resume/live 重建：空 aborted assistant 投影为标记条目（无气泡、无 Aborted 通知）；
  部分内容 aborted assistant 投影维持现状；单测覆盖两种形状。
- [x] T2.2 分段：仅含标记的簇不画簇摘要头；abort 轮次形成信封；单测覆盖 resume 回放帧。

## T3 transcript：信封与时长恢复接线

- [x] T3.1 ingest_rebuild_clocks 以标记恢复轮次末端；不足 1s 写 <1s、缺端省略沿用现行规则；单测覆盖亚秒 abort 轮。
- [x] T3.2 app-tui-transcript BDD：新增「abort 轮次信封带时长」场景与步骤绑定（场景构建器回放 abort 轮次并重建），跑 `cargo nextest run --all-features --test bdd --profile agent` 绿。

## T4 门禁

- [x] T4.1 `just fmt`、`cargo clippy -p xylitol-runtime -p xylitol-app-tui --all-targets`、两 crate `cargo nextest` 全绿；uncomment 闸过。实跑（2026-11-27，本分支）：nextest 两包 1300 passed；BDD `--profile agent` 1243 passed；clippy 无警告；`cargo fmt --check` 通过；uncomment 闸过。
- [x] T4.2 实跑证据落 tasks 备注后收口（archive 由流水线执行，不列为任务）。证据见 T4.1 备注；validate --strict 在本任务勾选后重跑。
