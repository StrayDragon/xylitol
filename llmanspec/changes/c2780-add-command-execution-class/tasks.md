# Tasks

> apply 修订：t2–t4 的 drain 接入被借用现实阻断（design §3），整体移交
> `c2790-invert-bash-dispatch-ownership`（已 draft，depends_on 本 change）。
> 本 change 交付范围收缩为 t1（Exec SSOT）。

- [x] t1: Exec 类 SSOT——REGISTRY Exec 维度 + `exec_class` 无通配穷举推导 +
  守卫测试（`exec_columns_lock_declaration` 分类列快照锁、
  `exec_class_matches_registry_declaration` 代表对照）。
  验证：`cargo test -p xylitol --lib protocol::wire::registry`（6/6 绿）。

## 移交 c2790（原 t2–t4，含已落 spec 场景的回填）

- t2: `drain_inline_pending` 窄入口 + bang select drain 臂 + BDD 绑定
  （场景 bang-inline-effect-runs-during-bang / exclusive-stays-queued——
  ath45 规则与场景自 c2780 landing 移交，c2790 重新落地）。
  前置：bash 派发所有权倒置（XyDriver owned 完成接收端，镜像 run()→EventStream）。
- t3: `/model` / `/thinking` Inline 生效路径行为验证 + PTY 时序冒烟
  （bang 不结束时 `→ * fake` 即挂载；Esc 即关；/exit 干净）。
- t4: reload 循环同臂（`reload_runtime` future 可同型倒置则做，否则注明只做 bang）
  + `just qa` 全绿。
