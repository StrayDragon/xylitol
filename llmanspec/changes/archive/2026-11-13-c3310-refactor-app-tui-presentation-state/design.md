# Design — app-tui 呈现层状态收敛

## 0. 复核结论（2026-11-13，HEAD 7d36d7e 逐条开文件核对）

14 点位：成立 13、表述需修正 1（D10）、不成立 0、已修复 0。

重叠裁定：c3275 收敛了 host busy 真值（HostBusy + recalc_busy），但其文案回落仍按 "Working"/"Running"/"Reloading" 字面量判 stale（host/mod.rs:491-494）——D9 未修复且多一处新文本反推点；c3280/c3285/c3290/c3295 无重叠；c3300 的 `BashOutcome` 枚举（sdk ports/bash.rs:19-24）可直接复用为 D6 修复 seam。

修正一处：D10 实际形状 = ScrollbackFold **10 bool + 1 ArgsMode 枚举 + 5 per-id override map**（widgets/scrollback/mod.rs:45-63，扫描日即如此）；实质成立（全局默认 × override 双真相 + ActivityFold 整批覆写）。

关键成立点（证据）：D6 逆序定位无 id（model.rs:470-486/535-558；Compaction 无 id 字段，lifecycle.rs:146-152/177-192），Cancelled 覆写过宽（model.rs:552 允许改写 Success/Error）；D7 实为**三写入口**（事件 lifecycle.rs / slash compact.rs:32-68 / 重建 session_tree.rs:242-248），去重仅 slash 路径且按 tokens_before 相等误判；D8 七段顺序 if（driver/types.rs:111-137）；D9 cue 前缀嗅探 + 字符串清除（root/mod.rs:521/526-532/815-819、1519-1527）；D12 loading/rename/confirming_delete 组合无定义行为（session_resume/panel.rs:36-46/:100-108）；D13 EXIT_REQUESTED 全局 AtomicBool 无复位（terminal_guard.rs:6/:16-18）；D18 拖拽仅 Up 置 false（render.rs:368-412 唯一写点 :403）；D20 aborted 由通知文本反推（model.rs:30-39 字面量）——强于扫描表述；D22 take_submit 不清 submit_attachments、converted_submit 不取残留（pending.rs:8-34、effects/mod.rs:281-286）；D24 查询形 take_* 多消费点先到先得（root/mod.rs:791-793/1008-1014、pending.rs:54-55）；D25 AskPhase 由 result status 字符串反推、非 JSON 回落 Answered（preview.rs:262-284）；D21 boot spinner 成立但模块自包含有单测，降级为可选任务。

## 1. 目标与边界

- 呈现层状态收敛；交互行为与视觉输出不变（needs_specs_change: false）。
- 边界：不动 layout 壳与 app-tui-fixed-zone 固定词；cue/busy 收敛不改变对外文案值。

## 2. 决策摘要

| 项 | 方向 |
|---|---|
| D6 | Bash/Compaction 块按 id 定位（Compaction 补 id）；Cancelled 改写仅限 `*st == Pending` |
| D7 | 三写入口收敛为单一 upsert（复用 D6 块 id；废除 tokens_before 相等判重） |
| D8 | ReadyState 显式分解（NotConfigured/Connecting/BootstrapPending/Frozen/Arming/Ready），消费点不认序 |
| D9 | Cue 枚举；status_busy 从 UiPhase 推导；recalc_busy stale 判据去字面量 |
| D10/D11 | ActivityFold VisibleItems 派生化（去字段、删 effective_level 别名）先行；ScrollbackFold defaults+overrides 单一结构，visible 字段单向流入 |
| D12 | PanelSubstate 枚举（Normal/Rename/ConfirmDelete）；set_loading 复位子状态 |
| D13 | EXIT_REQUESTED 收进 session 生命周期或补复位 |
| D18 | 拖拽补收尾（slot 打开 / Esc / placement None）+ Drag 边界检查 |
| D20 | StageNamePhase 枚举；aborted 改从 AgentEnd reason 取 |
| D22 | 单槽 `Option<PendingSubmit{prompt,attachments}>`，gated 为状态 |
| D24 | take_* 拆只读 + 显式 clear，消费收敛到 paint 单点 |
| D25 | AskOutcome 类型化（类比 BashOutcome），非 JSON 显式 Unknown 不默认 Answered |
| D21 | 可选任务（可裁） |

## 3. 实施顺序

T1–T4 同文件族串行（model/driver/root）；折叠两单 T5→T6 有依赖；其余可并行。见 tasks.md。

## 4. 测试边界

单测（状态机矩阵 / 真值表）+ 既有 harness 快照；行为不变由 app-tui BDD 守住。
