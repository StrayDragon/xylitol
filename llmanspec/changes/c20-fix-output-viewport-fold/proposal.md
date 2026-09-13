---
depends_on: []
needs_specs_change: true
rules_touched:
- att30
- peo1
- peo3
branch: tui/tool-output-fold
base_sha: 8e97f1ffe2dab1e22780d9f7efe0e8b0b93b2992
---

## Why

产品 TUI 中点击某个工具输出块的 `... (N earlier lines, ctrl+o to expand)` 提示带，按现行合约（att30）与实现是翻转**全局** `tools_output_expanded`，导致所有带该提示的块（Tool / Bash / Diff）同时展开，与「只展开我点的这一块」的直觉相悖。且块展开后没有任何可见可点的折回锚点，只能再按一次 Ctrl+O 把全部块一起折回，粒度同样过粗。

## What Changes

- 输出视口（Ctrl+O 语义）从单一全局布尔改为「全局默认 + 按块覆盖表」（镜像既有 `tools_overrides` / `thinking_overrides` 模式）：`ScrollbackFold` 新增 `output_overrides` 与 `output_effective(id)`；`FoldTarget::OutputViewport` 携带块 id；点击某块的展开提示带只翻转该块。
- 键盘 Ctrl+O（`app.tools.expand`）保留全局翻转语义并清空按块覆盖（与 Alt+E / Ctrl+T 的「默认+覆盖」模型同构），继续提供全部展开 / 全部折叠的逃生口。
- `packages/xylitol-tui` 的 `render_expandable_output`：expanded 且内容视觉行数超过 `max_preview_lines` 时，块尾渲染 dim 折叠提示行（`... (expanded, {fold_hint})`，默认 `ctrl+o to fold`），新增 `fold_hint` 选项；内容未超上限或为空时不渲染。app 侧在该行注册可点击命中，单击独立折回该块。硬截断块（att16）永不展开，因此天然无此行。
- `UiEntry::Bash` 补稳定 `id`（直播路径分配序号 id；rebuild 路径取 session JSONL 的 `bashId`），作为 Bash 块的按块键。

## Capabilities

- `app-tui-transcript`（att30：提示带点击从全局翻转为按块翻转；新增 fold 提示带命中）
- `package-tui-expandable-output`（peo1：选项表加 `fold_hint`；peo3：expanded 条件性 fold 提示行）

## Impact

- 代码：`src/app/tui/widgets/scrollback/{mod,diff,paint,cache}.rs`、`src/app/tui/widgets/fold_hit.rs`、`src/app/tui/layout/root/{mod,slot_input}.rs`、`src/app/tui/bridge/model.rs`、`packages/xylitol-tui/src/components/expandable_output.rs`。
- 行为：点击/折叠交互粒度从全局变按块；全局键位语义不变；硬截断行为不变（att16）。
- 缓存：per-block effective 视口进入 entry fingerprint，维持 ath25「只重绘受影响块」性质。
