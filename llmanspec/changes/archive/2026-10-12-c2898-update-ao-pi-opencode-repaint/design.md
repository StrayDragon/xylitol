# Design — c2898 AO 交互卡顿（stale 粒度细化）

## 前置：调研结论（research/latency-findings.md）

滚轮管线健康（release 12us）；卡顿主嫌=**busy 期 spinner tick→`ao_components_stale`全树重排**（dev 18.4ms×60Hz）。本票不动滚轮链路。

## D1 stale 粒度模型（核心拍板）

现状：`TUI::tick` 任组件 `changed`→`ao_components_stale=true`（全树重 render + dock 重测 + 全量 finalize）。

目标：**动画级 dirty 与结构级 stale 分离**：

```
enum AoDirty { None, Repaint, Restale }
```

- `Component::tick` 返回值细化为 `AoDirty`（默认 Repaint 兼容）：spinner/时钟等**纯显示动画**→`Repaint`（仅重投影 + dock 行重排，**不**重 render 组件树、**不**重测 dock）
- 内容结构变化（流式 delta/fold/resize）→`Restale`（现行为）
- `can_reproject` 的 `!ao_components_stale` 条件改接受 Repaint 后的帧？——**否**：Repaint 帧走**dock-only 重排**新路径：上帧 `component_lines` 缓存中仅重排 dock 段 + transcript 原样复用（wrap 不变），再 project/finalize。规避全树 render 的 5.1ms。
- **一步到位（用户拍板 2026-10-12）**：`Component::tick` 签名**直接改为** `fn tick(&mut self) -> AoDirty`，全仓 Component 实现一次性迁移，**不留** `tick_dirty` 双轨/兼容层。迁移时逐组件判定：纯显示动画（spinner/时钟/进度）→`Repaint`；内容/结构变化→`Restale`；拿不准先 `Restale`（保守=行为不变），后续票再细化。
- 风险：dock 高度随动画变（spinner 行数不变✓）；transcript 区动画组件误判 Repaint 会跳过其内容重排——判定清单进 PR 描述，逐个核对。

## D2 诊断基建

- 修 `lab_ao_stream_delta_perf_report`（`on_run_started` 一行，F6）
- 新 lab 场景 `lab_ao_busy_mixed_perf`：spinner tick + wheel 交错，输出 p95 与重排次数——E1 前后对比 + 回归守门（`#[ignore]` lab，不进 qa）

## D3 验收

- busy 混合负载下 wheel p95：dev <5ms、release <1ms（现 dev 全重排 18ms）
- 既有全测绿；`ao_reproject` 比率不降
- E2：产品 run 路径确认 release（`just run` profile 检查），dev 提示措辞入 docs?（仅确认，不改行为）

## 已决（2026-10-12 用户）

- 签名一步到位（见上），不留兼容层。
- E3：idle 与 busy 均卡（用户实证）——D1 为全局结构改造，非 busy 专属补丁。

## 实作订正（T2 实测后，偏离 D1 原案）

**未引入 `AoDirty`/签名改造**——T2 实验发现根因更浅：`idle_tick` 里「动画 dirty→`ao_components_stale=true`」这一行**冗余**（dock 每帧现算天然带新 spinner 帧；upper 区走 gen 缓存；内容/结构变化各处本就显式 mark）。删除该置位后：

| busy-mixed（dev） | 前 | 后 |
|---|---|---|
| wheel p50/p95/max | 916 / 1071 / **2632** | 924 / 1010 / **~1438（-45% 尾）** |
| tick p50/p95/max | 1217 / 1557 / 2617 | 1060 / 1233 / 1836 |
| release wheel p50/p95 | — | **294 / 367** |

偶发 2.6ms 尾延迟（卡感来源）收敛过半；D3 阈值（dev<5ms/rel<1ms）达标。**签名改造取消**：无新 API、无兼容面，比「一步到位」更彻底；AoDirty 显式化待未来真出现「需 Restale 的动画」再议（YAGNI）。

## E2 结论

`profile.dev.package."*"=opt3` 已有、本 crate 两 crate opt0——dev 3.6× 为既定构建取舍。产品日常建议 `--release`；不改默认（build 时间权衡，用户可调）。
