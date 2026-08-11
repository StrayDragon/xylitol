# Research: Fold Leader vs 全局 Alt+E

> Change: `c2030-add-tui-fold-leader-digit-toggle`

## 今日行为（代码）

| 动作 | 状态 | 作用域 |
|---|---|---|
| `app.tools.blocks` / `alt+e` | `tools_expanded` + `compaction_expanded` | **全局**所有 tool/diff；compaction 同和弦 |
| `app.thinking.toggle` / `ctrl+t` | `thinking_expanded` | 全局 thinking |
| `app.tools.expand` / `ctrl+o` | `tools_output_expanded` | 全局视口高度 |
| 会话树 fold | `folded_nodes` | **per-id**（可借鉴，非 scrollback） |

真源：`src/app/tui/widgets/scrollback.rs` `ScrollbackFold`；`src/app/tui/keybindings.rs`。

`c1760` 已定性：**三个全局 bool，不是 per-entry**——「定点折叠」必须先还债。

## 与用户意向对齐

用户：`Alt+E` → 编号高亮 1…0 → 数字只 toggle 对应块。

这与今日「一键全局翻转」**冲突**。候选：

| 方案 | 优点 | 缺点 |
|---|---|---|
| A. Alt+E 改为 leader-only；全局另绑 | 符合用户描述 | 破坏肌肉记忆；需文档/旁注 |
| B. Alt+E 仍全局；新和弦进 leader | 兼容 | 多学一键 |
| C. Alt+E 单击全局 / 短时再按进 leader | 紧凑 | 时序难测、易误触 |
| D. Alt+E 进 leader；无数字时 Esc；双击或 `Alt+Shift+E` 全局（注意与 c1760 段栈和弦冲突） | 可折中 | 和弦拥挤 |

**调研倾向**：propose 时优先 **A 或 B**；避免 C。与 `c1760` 的 `Alt+Shift+E` = `activity.expandNearest` **必须错开**。

## Per-block 状态模型（意向）

```text
global_default: tools_expanded = true（今日默认）
overrides: HashMap<EntryId, bool>   // 或缺省跟 global

render(entry):
  expanded = overrides.get(id).copied().unwrap_or(global_default)
```

- 「全局 Alt+E」（若保留）可清空 overrides 或只改 default——须钉一种，避免「看起来没反应」。
- fingerprint / paint-cache MUST 含 overrides。

## Leader 模式细节

- **编号序**：距输入最近（列表尾）= 1，与 c1760「从底剥」心智一致。
- **封顶 10**：`1`–`9`、`0`=10；第 11+ 不可达 → 滚动后再开 leader，或显示「+N more」提示（非 MUST）。
- **超时**：可选 3–5s 自动退出；Esc 立即退出；**MUST NOT** 吞后续普通输入超过模式寿命。
- **Thinking**：建议首波 **不进** 同一编号平面（仍 Ctrl+T），降低复杂度；Open Question。

## 性能

- 进入 leader：扫描**可见**折叠头建表 O(v)，v≪总 entry。
- 高亮：复用既有行 + 覆盖样式，或只重画头行；禁止全 scrollback Markdown 重解析。
- 与 ath25：toggle 单 entry → 该条及后续高度失效即可。

## 跨面

动作语义意向：`fold.enterLeader` / `fold.toggle(target)`；Web 可用点击代替数字。物理 `Alt+E` 不写进 Web MUST。
