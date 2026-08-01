# Design：ath12 复杂度闸

## 问题

文件 LOC 与「难维护」相关但不等价。ath12 需要：

1. **结构边界**（已有，保留）
2. **防入口协调者再堆回巨型控制流**（应用函数级复杂度，非行数）
3. **行数**仅作导航/嗅觉辅助

## 指标选择

| 候选 | 取舍 |
|---|---|
| Clippy `cognitive_complexity` | 无 cyclomatic；restriction 默认 off；与 cccc 分数漂移；开闸需大量 allow |
| lizard CCN | 仅 McCabe；Rust 嵌套深度不可靠；测例 NLOC 噪声 |
| rust-code-analysis | crates.io 安装现不可用 |
| **cccc-rs** | Sonar cognitive + McCabe；`--max-*`；JSON；CLI 非产品 dep → **主闸** |

## 分层（与实现一致）

```text
HARD qa  Clippy -D warnings（不含 cognitive_complexity）
HARD qa  check_complexity.py → 四入口 cognitive≤35 / cyclomatic≤30
SOFT     just complexity --radar（子树 top-N）
SHOULD   入口物理行显著低于 ~800；逼近 ~1200 优先拆分
```

## 为何入口阈值 35/30

当前四入口：`step` 33/28，`apply_tool_result` 30/25。阈值略高于现状，避免无意义抖动，又挡住明显回潮。收紧须另开 harness 变更并先拆烫点。

## UiRoot::new 噪声

cccc 对几乎无分支的构造可报偏高 cognitive。入口闸以 **max over functions** 过线；`new` 现 17≪35，暂不特殊排除。若误伤再加 exclude 名单（须写进脚本与本 design）。

## 合约迁移

- ath12 statement：删「各 MUST 显著低于约 800 行」硬句；改为 SHOULD + 1200 硬味 + 复杂度 MUST
- feature `god-files-under-budget` → `entry-complexity-under-budget`（名字可微调）
- `test-qa-gate` qg06：点名 `check_complexity.py` 属 check_* 契约
