# 设计：压缩度量口径一致与校准

## 现状（实机证据）

- 三处度量互不同源：chars/4（cut）、tokenizer+fixed_context（footer/触发）、provider 实测。
- r1406 已锁「触发同源、禁 len/4」——但 cut/守卫路径用的是 len/4 类启发式，脱离纪律。
- 观测分歧 2x（footer 35k vs provider 17.4k），方向待定位（下述 lab）。

## 方案

1. **切点/守卫并入统一估算入口**：
   - `effective_keep_budget`（已用 settings/window 计算）配合 `find_cut_point` **用 `estimate_tokens_entry` 的统一口径**（经 `EstimateOpts`/tokenizer 闸，与触发/footer 同一函数族），移除 `estimate_tokens_entry_for_cut` 的独立 chars/4 作为默认。
   - lax 兜底（反序列化失败）保留但标记非 SSOT。
   - 切点热路径成本：tokenizer 编码较重 → Depth-First 只在必要时启用（allow_local_tokenizer 闸），否则退化统一启发式并与其校准系数同源。
2. **校准对拍 lab（先只读）**：
   - `estimate_from_session_entries(0664dab6)` vs `llm.request input=17441` 实测对照；
   - 逐项拆分：tokenizer 差异、fixed_context 是否重复、usage 锚点历史叠加是否双计；
   - 输出偏差系数与误差边界，落成 lab 文档（进 c2848 research/）。
3. **spec r1924**：产品级 WHAT——压缩决策链（切点、触发、展示）MUST 对同一替换后上下文使用同一可校准度量源；任何度量系统性偏差 MUST 可解释；切点决策 MUST NOT 依赖与展示矛盾的独立启发式。
4. 回归：单测/BDD 锁「同一数据上 cut 累计 == estimate 同源结果」；lab 校准对照触发（人跑维护）。

## 明确不做（本轮）

- turn-end 双发 skipped 重构（用户暂缓）。
- 不改 provider 侧；不改 reserve/keep 常量（除非校准证明需要）。
