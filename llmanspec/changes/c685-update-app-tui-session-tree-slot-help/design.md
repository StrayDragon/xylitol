# Design — c685-update-app-tui-session-tree-slot-help

## 命名

- **用**：树槽 Search/Help、layout、`EditorSlot::Tree` 头行
- **不用**（新文案）：chrome（易与浏览器混淆）
- **例外**：合约 id `app-tui-chrome` 历史保留；口语/新 docs 写「layout 壳」

## 决议

1. **槽头归属产品 layout**：标题 `Session tree` + Search 行 + Help 行 + `TreeSelector::render`；不把产品文案写进包 `TreeSelector` 默认 render。
2. **Help 数据源**：键 id 对齐 pi 语义（move=`tui.select.up/down`；page=`tui.select.pageUp/Down`；branch=`tui.tree.foldOrUp/unfoldOrDown`；filters=产品 Ctrl+D/T/U/L/A；cycle=forward+backward）。用户改键后 Help **跟着变**。
3. **Search 行**：空 → muted `Type to search:`；非空 → `Search: {query}`。
4. **cycleBackward**：默认 `ctrl+shift+o`；树开优先。
5. **Label 键在 Help 中**：MAY 显示 Shift+L/T；persist 属 c690。
6. **文档**：`session-tree-vs-pi.md` 表内 fork / 槽头 Help / cycleBack 真值。

## 验证（Agent 自验 → 人类确认观感）

| 层 | 谁 | 命令 / 路径 | 期望 |
|---|---|---|---|
| harness | Agent **必跑** | `cargo test --lib -- h21_tree_slot h22_tree` | Search 回显；Help 含 filters/cycle；Ctrl+Shift+O → `[all]` |
| fmt/clippy | Agent **必跑** | `just fmt`；相关 clippy | 绿 |
| SDD | Agent **必跑** | `llman sdd validate c685-…-slot-help --strict` | 绿 |
| 产品 Fake 手测 | **人类确认**（Agent 可先开） | `cargo run -- --trust --tui --model fake`（隔离 HOME 见 just 注释） | 双 Esc → 见 `Type to search` + Help；键入搜索回显；Ctrl+Shift+O 状态行 `[all]` |
| demo 对照 | 可选 | `just demo-tui` 双 Esc | 形态参考，非产品合约 |
| PTY/tmux | Agent 在 **c705** 落地；本 change 不挡 | `just test-tui-e2e-pty` | c705 场景覆盖树槽 |

人类只确认「看起来是否像 pi / 是否可读」；回归与修 bug 仍由 Agent 自修后再交。

## 权衡

| 方案 | 取舍 |
|---|---|
| 包内通用 TreeHelp 组件 | 开闭好，但产品键名混杂；本 change **优先产品函数** |
| 继续硬编码 | 否决 |

## 非目标

- `app.tree.filter.*` 全量迁到 KeybindingsManager（MAY follow-up）
