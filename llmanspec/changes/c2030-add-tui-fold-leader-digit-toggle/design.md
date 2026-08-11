# Design: c2030 Fold Leader（α′）

## D1 状态

```text
UiRoot
  fold: ScrollbackFold                 # 全局默认（thinking / tools / viewport / compaction）
  tool_block_overrides: Map<TargetKey, bool>
  fold_leader: Option<FoldLeaderMode>  # None | { map: digit→TargetKey, built_at_gen }
```

- `TargetKey`：Tool/Ask 用稳定 `id: String`；Diff 用合成键（如 `diff:{entry_idx}` 或内容指纹）——实现选一种可测稳定方案。
- 渲染展开：`overrides.get(k).copied().unwrap_or(fold.tools_expanded)`。
- 全局 `Alt+Shift+B`：`tools_expanded ^=` **且** `overrides.clear()`。
- Compaction：只读/写 `fold.compaction_expanded`（`Alt+Shift+C`）；不进 leader 平面。

## D2 FoldLeaderMode

1. `Alt+E`：若已在 mode → 退出（或刷新映射——**钉：退出**）；否则扫描当前视口可折叠头（Tool/Diff/Ask），按距输入近→远填 `1…9,0`，进入 mode，局部重绘头行编号。
2. digit：toggle 对应覆盖；退出 mode（**钉：toggle 后退出**，降低误触连按）。
3. Esc：退出，不改 fold。
4. 其它可打印/编辑键：退出 mode，**同一 Key 仍送达 Editor**（listener Continue 或 clear-then-redispatch）。
5. **禁止** `Alt+digit` 作为默认绑定（Ghostty 切 tab）。

## D3 视口扫描

无引擎「可见折叠头」API → 产品用当前 scrollback 行列表 + viewport 窗（或等价 content-end 窗）估计可见 entry 头行。O(可见 entry)。进 leader / 退出只 bump 必要 gen；**禁止** `scrollback_paint.invalidate()` 全表除非全局 fold 真变。

单块 override 变更：更新该 entry fingerprint，自该 idx `truncate` 重画（ath25）。

## D4 键位 id

| id | 默认和弦 | 行为 |
|---|---|---|
| `app.tools.foldLeader` | `alt+e` | 进/出 leader |
| `app.tools.blocks` | `alt+shift+b` | 全局 tools + 清覆盖 |
| `app.compaction.toggle` | `alt+shift+c` | 全局 compaction |
| `app.thinking.toggle` | `ctrl+t` | 不变 |
| `app.tools.expand` | `ctrl+o` | 不变 |

热重载仍走 `ati35`。

## D5 与邻居

- c1760：旁注 L2 行继续 `(Alt+Shift+E)`；本 change 文档声明不占用。
- c2040：点击写同一 `tool_block_overrides`；动作语义 `fold.toggle(target)`。
- agent_demo：`att8` 将改为产品键语义或注明 demo 可滞后——specs 以产品为准，demo 跟随 tasks。

## D6 非目标

鼠标、段级编号、thinking leader、随滚重标。
