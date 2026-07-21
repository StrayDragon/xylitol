# src/ 体量调音（易腐）

> **性质**：易腐调音表，**不是**架构规范。稳定边界见 `src/AGENTS.md`；调优待办见 [`_TODO.md`](./_TODO.md)。
> **谁改**：拆文件 / 合并后超硬顶 / 季度调音时更新本表；**不要**把本表抄进 AGENTS 正文。
> **行数命令**：`find src -name '*.rs' -exec wc -l {} + | sort -nr | head -20`

## 预算

| 类别 | 软顶 | 硬顶 | 说明 |
|---|---|---|---|
| 生产模块（`src/**/*.rs`，非测试专用） | ~1200 行 | ~2000 行 | 超软顶 → 规划拆分；超硬顶 → 合并前应有拆分计划或 RETUNE 注明豁免理由 |
| 测试专用（`harness.rs` / `tests.rs` / `#[cfg(test)]` 大块） | — | 另计 | 允许更长；仍禁止无结构堆叠——按场景/模块拆文件优于单文件无限长 |
| `packages/` | 本表不覆盖 | — | 见各包 `AGENTS.md`；本轮调优优先 `src/` |

「行」= `wc -l` 物理行（含空行与测码同文件内联时一并计入生产文件——故优先把大块 `#[cfg(test)]` 外置）。

## 触发调音

任一条成立即更新本文件「超标表」与「调音记录」：

1. 合并后某生产文件 **> 硬顶**；或
2. 自「上次调音」起满约 **一个季度**；或
3. 完成 [`_TODO.md`](./_TODO.md) §D 一类拆分 PR 后（核对是否仍超标）。

## 超标表（生产模块）

快照日期：2026-07-21（c1220 后 `wc -l` 重测）。权威以现场 `wc -l` 为准。

| 文件 | 约行数 | 相对硬顶 | 拆分入口 |
|---|---|---|---|
| `agent/runtime/react.rs` | 2775 | 超硬顶 | **默认不拆**（同居行为测 ~1500；真剧本 ~780）。见 `_TODO` §D 决议；P3 未开闸 |
| `infra/session/manager.rs` | 2090 | 超硬顶 | **默认不按 D8–D10 大拆**；D11 已做。见 `_TODO` §D |
| `app/core/driver/in_process.rs` | 1265 | 超软顶 | §D5–D7 已拆；仍可再切 reload/clipboard/tests |
| `infra/resource/loader.rs` | 1217 | 超软顶 | 暂观察；有改动时顺手拆 |
| `infra/config/types.rs` | 1190 | 近软顶 | 暂观察（类型清单型文件） |
| `agent/session/mod.rs` | 1056 | 近软顶 | 暂观察 |
| `app/core/bootstrap.rs` | 1007 | 近软顶 | 暂观察 |

`app/core/driver/` 拆后：`types` 227 / `proto` 260 / `remote` 753 / `mod` 41；合计约 2546，单文件已无 >2000。

c1220 后 `protocol/` 根模块（均未超软顶，知情）：`session` 820 / `types` 653 / `message` 457 / `lifecycle` 159 / `error` 137。勿为行数再抽 `xylitol-domain`。

## 测试专用（另计，仅知情）

| 文件 | 约行数 | 备注 |
|---|---|---|
| `app/tui/harness.rs` | 4144 | 可按场景族拆文件；不与生产硬顶混用 |
| `app/tui/tests.rs` | 1884 | 同上 |

## 调音记录

| 日期 | 说明 |
|---|---|
| 2026-07-21 | 初建：预算 + 超标快照；对应 `_TODO.md` §A |
| 2026-07-21 | §D5–D7：`driver.rs` → `driver/` 子模块；重测行数 |
| 2026-07-21 | 决议：react / session manager **默认不大拆**；超标保留并注明理由（见 `_TODO` §D） |
| 2026-07-21 | D11a+b：EventBus port impl 归 `infra/event`；删除孤儿 `session/tests.rs` |
| 2026-07-21 | c1220 后重测：react 2775 / manager 2090；protocol 根模块均 < 软顶；F8 domain 包搁置 |
|  |  |

## 下次调音

- 建议不晚于：**2026-10-21**（约一季度），或 §D 首批拆分合并后立即重测行数。
