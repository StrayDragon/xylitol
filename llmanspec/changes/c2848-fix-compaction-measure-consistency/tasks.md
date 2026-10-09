# Tasks

## T1 — 校准对拍（只读，先于实现）
- [ ] 对 0664dab6 深会话：`estimate_tokens_entry_for_cut` 累计 vs `estimate_from_session_entries` vs provider `llm.request input`（17441）三数对照。
- [ ] 拆分 2x 来源：tokenizer 口径 / fixed_context 叠加 / usage 锚点重复。
- [ ] 产出偏差系数 + 误差边界 → 落 `llmanspec/changes/c2848.../research/` 文档（lab_ 或研究笔记）。
- 校验：文件存在且数据自洽（provider 实测为锚）。

## T2 — 切点/守卫度量并入统一入口
- [ ] `find_cut_point` / `prepare_compaction` 度量改走统一估算（EstimateOpts/tokenizer 闸），移除 chars/4 默认；lax 兜底标记非 SSOT。
- [ ] 热路径成本：tokenizer 懒加载 / allow_local_tokenizer 闸；必要时 spawn_blocking。
- [ ] 既有 cut_detector 单测全量过（切点语义不因口径变化回归）。
- 校验：`cargo test --lib agent::compaction` 绿。

## T3 — spec r1924 + 回归
- [ ] domain-compaction.feature 落 r1924（产品级 WHAT：决策链统一可校准度量；偏差可解释；切点不得用独立启发式作 SSOT）+ 场景。
- [ ] BDD/单测锁「同一数据上 cut 累计 == estimate 同源」。
- [ ] `check_spec_anchors` / `llman-sdd validate --strict` 绿。
- 备注：turn-end 双发 skipped 本轮不重构（用户暂缓，草案证据保留）。

## T4 — 门禁收口
- [ ] `just fmt` / `lint` / `test`（nextest）/ `doc-check` / validate 全绿。
- [ ] `llman-sdd change finalize c2848-... `（不带 --no-check）+ push。
