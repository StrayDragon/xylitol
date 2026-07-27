# Design: c1630-update-compaction-reserve-formula

## 目标边界

| 在范围 | 不在范围（交给后续 change） |
|---|---|
| 触发公式 = pi `tokens > window - reserve` | c1640：turn 后 auto / manual force |
| 删除 `compaction_threshold: f64` 全路径 | c1650：assistant 切点 / split-turn |
| 配置 SSOT = 三字段；文档/合约对齐 | c1660：overflow retry |
| `get_context_usage` 的 `should_compact` 吃 settings | c1670：optional instructions |
| BDD/单测验收与公式一致 | c1680：TUI % 条（percent 派生展示可先保留字段） |

## 公式（SSOT）

```text
should_compact =
  settings.enabled
  && context_window > 0
  && context_tokens > context_window.saturating_sub(settings.reserve_tokens)
```

与 pi `shouldCompact(contextTokens, contextWindow, settings)` 同构。
`keep_recent_tokens` **不参与触发**，只服务切点（既有 `find_cut_point`）。

## API 形状

### 选定

```rust
// orchestrator / pub re-export
pub fn should_compact(
    context_tokens: u64,
    context_window: u64,
    settings: &CompactionSettings,
) -> bool;

// session stats — percent 仍为派生展示，不进触发
pub fn get_context_usage(
    token_estimate: u64,
    context_window: u64,
    settings: &CompactionSettings,
) -> ContextUsage;
```

### 拒绝

| 方案 | 原因 |
|---|---|
| 保留 `threshold: f64` 与 reserve 双闸 | 两套 SSOT |
| `should_compact(..., reserve_tokens: u64)` 散参且忽略 `enabled` | 易漏关断 |
| 删除 `ContextUsage.percent` | 属 c1680 展示；本 change 只改 `should_compact` 来源 |

### 删除清单（验收用）

生产路径零残留：`compaction_threshold` 字段/方法（builder、composition、bootstrap、`AgentCapabilities`、fixtures 等）。
测试夹具改传 `CompactionSettings`。
`should_compact_by_reserve` 与公开 `should_compact` 合并（勿两套同名语义）。

## 配置

已有 `XyCompactionSettingsConfig`（camelCase）→ `CompactionSettings`：

| 字段 | 默认 |
|---|---|
| `enabled` | `true` |
| `reserveTokens` / `reserve_tokens` | `16384` |
| `keepRecentTokens` / `keep_recent_tokens` | `20000` |

**禁止**配置/schema 再引入 `threshold` / `compaction_threshold`。
示例 `.xylitol/config.yaml` 可显式写出三字段（文档化默认，非行为变更）。

c14 文案：运行时类型名统一为 **`CompactionSettings`**（非 `XyCompactionSettings`）；配置类型仍为 `XyCompactionSettingsConfig`。

## Orchestrator

`CompactionOrchestrator` 去掉内嵌 `threshold: f64`，只持 `CompactionSettings`；`maybe_auto_compact` 调用新 `should_compact(..., &self.settings)`。
（真正「何时调用 maybe」仍属 c1640；本 change 只改判定尺子。）

## 估计同源（c16 不变精神）

触发所用 `context_tokens` 仍必须来自 `estimate_from_session_entries` / 同源 helper（c1/c16），**不得**为触发另算独立 `len/4` 总和。
本 change 只替换「与谁比较」：从 `tokens/window >= 0.8` 改为 `tokens > window - reserve`。

## 文档

`docs/architecture/压缩与上下文.md`：

- 删除「默认约 80% 窗口」；
- 写明触发 = `占用 > 窗口 − reserveTokens`；
- 百分比若出现，标明为**派生展示**，非触发配置；
- 「理想 vs 现状」中自动压缩接线仍可指向后续（c1640），但公式心智本 change 先对齐。

## 测试 seam（验收边界）

复用既有 harness，不新造脱离 `.feature` 的边界：

| # | Seam | 覆盖 |
|---|---|---|
| 1 | `domain-compaction.feature` → `should_compact` / settings | `need-compact` / `no-compact` / `disabled-no-compact`；c2 |
| 2 | 同 feature → 估计同源 | `threshold-shares-footer-estimate` 改写为 reserve 触发 + 同源估计（c2+c16） |
| 3 | `runtime-config.feature` `@req:rc15` | YAML `keepRecentTokens`（及可选 `reserveTokens`）映射到运行时 settings；**禁止**断言已删除的 `.threshold` |
| 4 | 单测 `src/agent/compaction` | 公式边界：`tokens == window - reserve` → false；`+1` → true；`enabled=false` → false；`window=0` → false |
| 5 | 全仓 grep 闸（task 验收） | 零 `compaction_threshold` 生产残留 |

## 迁移注意

- 旧 BDD「压缩阈值为 0.8」步骤废弃，改为 `reserveTokens` / settings Given。
- `AgentCapabilities::new(..., 0.8, ...)` 等签名删除 threshold 参数——机械改动面大，task 按「先改 API 与调用点，再删旧」排序。
