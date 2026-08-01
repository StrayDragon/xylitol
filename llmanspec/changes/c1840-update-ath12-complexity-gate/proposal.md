---
depends_on: []
---

# ath12：入口复杂度硬闸，行数降为软味

> 已落地的 `scripts/check_complexity.py`（cccc-rs：cognitive≤35 / cyclomatic≤30）进 `just qa`；本变更把 **ath12 合约**与 **god-module 行数硬测**对齐为「结构 MUST + 复杂度 HARD + 行数 SHOULD」。

## Why

ath12 的「入口显著低于约 800 行」是防回潮的粗代理：同一行数可以是很多薄方法，也可以是几个巨型 match。实测 `layout/root` 逼近 850 悬崖而 maxCCN 仍低，而 `input_policy` / `bridge` 行数安全、复杂度已烫。行数硬测会驱动「挪体积」式拆分，而非对准难读函数。

## What Changes

- **ath12**：保留模块边界 / 单 `drain_pending` / 禁组件触 Driver 等 **结构 MUST**
- **行数**：从 MUST「显著低于约 800」降为 **SHOULD**；逼近约 1200 仍视为硬味，**MUST** 优先拆分（语义保留，不再用 850 硬 assert 当合约实现）
- **复杂度 HARD**：ath12 入口协调者（`host/mod.rs`、`layout/root/mod.rs`、`effects/mod.rs`、`bridge/mod.rs`）函数级 **Sonar cognitive ≤35** 且 **McCabe cyclomatic ≤30**，由 `scripts/check_complexity.py`（cccc-rs）经 `just qa` → `check-scripts` 强制
- **test-qa-gate**：声明该脚本属于 `scripts/check_*` 满闸契约（qg06）
- **实现**：降级/删除 `god_module_entry_files_under_budget` 的 850 硬顶；场景 `god-files-under-budget` 改为复杂度闸（或拆场景）；阈值与脚本常量对齐

## Capabilities

- `app-tui-host`（ath12）
- `test-qa-gate`（qg06）

## 测试缝（apply 用）

| 缝 | 断言 |
|---|---|
| `scripts/check_complexity.py --check` | 四入口在 35/30 下绿；人为压低阈值红 |
| `just check-scripts` / `just qa` | 含 complexity 脚本且失败则整闸失败 |
| `god_module_entry_files_under_budget`（或后继） | **不再**以 850 为 ath12 MUST 实现；若保留行数测仅作 SHOULD/硬味 1200 |
| `app-tui-host.feature` @ath12 | `god-files-under-budget` 改写为入口复杂度；结构场景不变 |

## Impact

- 不再因 +数行被迫拆薄协调者；真正烫点靠 cognitive/cyclomatic 暴露
- Soft radar（`just complexity`）仍不硬失败，避免 slash/pending_ui 存量立刻炸闸
- Clippy `cognitive_complexity` 保持 ALLOW（与 cccc 分数不一致、全仓存量多）

## Out of scope

- 拆分 `handle_slash` / `drain_pending_ui` / `handle_slot_input`（雷达已标；另 change）
- 启用 Clippy `-D cognitive_complexity` 或 lizard 入闸
- 全仓（非 ath12 入口）复杂度硬顶
- 改动 `src/AGENTS.md` 1200/2000 体量启发式（可另文对齐）

## Open Questions

- （已拍板）HARD 指标 = cccc cognitive + cyclomatic；入口阈值 35/30（当前 max 33/28）
- （已拍板）行数 = SHOULD ~800 / 硬味 ~1200，去掉 850 硬测
