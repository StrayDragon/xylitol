# Tasks — 安全门禁 fail-closed

> 实施者导读：先读 design（含 §8 定稿记录与 spec delta 清单）。收口走 llman-sdd change finalize，不列为任务。

- [ ] T1 spec 失败语义条款先行（design §8）
  agent-hooks.feature r3 / r1006 / r1007 各增补「脚本执行失败（spawn/非零退出/非法 JSON/IO）按 Block 处理，reason 含失败分类」子句 + 嵌套场景（失败拦截）。验证：llman-sdd validate agent-hooks --strict 绿。
- [ ] T2 失败分类与档位推导（design §3 / §8）[blocked-by: T1]
  script.rs 失败路径返回失败分类（spawn_failed / nonzero_exit / bad_json / io_error；timeout 维持 Block）；dispatcher 按分发点类别（仅观察 / 强制）决定 Allow+warn 或 Block+reason；HookEntry 增 fail_open 逃生口。单测：失败矩阵 × 类别 × fail_open 全组合。
- [ ] T3 timeout 回归 r1010（design §8）[blocked-by: T1]
  移除 DEFAULT_HOOK_TIMEOUT_SECS 的 unwrap_or（未设 = 不限时）；新增可选全局 hooks.default_timeout_secs（未设 = 不限时）；改写 omitted_timeout_secs_gets_30s_default 测试。单测：未设不限时 / 全局默认 / 逐钩子覆盖三层。
- [ ] T4 匹配与分发收敛（design §2）
  phase_matches 单一实现（枚举与字符串共用）；typed 与 raw 两条 dispatch 循环提取公共骨架。验证：既有 dispatcher / merge 测试全绿；行为不变。
- [ ] T5 requires_approval 移除（design §8）
  config schema 删字段；两处 BDD 夹具清理。验证：rg requires_approval 全仓为空；just fmt / just lint 绿。
- [ ] T6 trust fail-closed（design §4）
  TrustOption 稳定 key（trust / trust_parent / deny）；ChoiceResult 回传 key；解析失败 / 未命中 / 越界 → Cancelled 直接退出。BDD：harness 注入垃圾答案与未知 key 场景。
- [ ] T7 观测与迁移文案（design §5）[blocked-by: T2]
  失败 Block 的 reason 与日志 error.kind；fail-closed 生效的迁移提示（一个 minor 版本）；release note 草稿（安全默认变更声明）。
- [ ] T8 全量回归 [blocked-by: T1, T2, T3, T4, T5, T6, T7]
  test-hooks-wiring / agent-hooks BDD 场景更新；just fmt / just lint / just test；llman-sdd validate --all --strict 绿。
