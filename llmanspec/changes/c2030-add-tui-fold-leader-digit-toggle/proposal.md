---
depends_on: []
blocks:
  - c2040-add-tui-mouse-click-fold-triangle
  - c2050-update-activity-fold-mouse-leader
---

# TUI 折叠块 Leader + 数字键定点 Toggle

> **一句话**：`Alt+E` 进入折叠块编号高亮模式（近→远 1…9/0），再按数字只 toggle 对应块；替代（或分层）今日「一键全局全开/全关」。

## Why

今日 `app.tools.blocks`（`Alt+E`）翻转全局 `ScrollbackFold.tools_expanded`——所有 tool/diff/compaction 块一起变。用户要「只收起这一块」只能全局切，体验粗。键盘无鼠标时，需要低摩擦的**定点**折叠。

意向交互（草案）：

```text
1. 用户按 Alt+E（或可配 leader）
2. 视口内可折叠块按「最新输出优先」标号 1,2,3…9,0 并高亮
3. 用户按对应数字 → 仅该块折叠/展开
4. Esc / 超时 / 再按 leader → 退出编号模式，焦点仍在 Editor
```

与 `c1760` 分层：本波先解决 **L1 块级**（tool/thinking/diff 等今日全局 bool 债）；段级 L2/L3 的编号/leader 由 `c2050` 与 `c1760` 对齐。

## What Changes

1. **状态机**：引入短暂 `FoldLeaderMode`（或等价）：编号映射 `digit → entry_id`；**MUST NOT** 抢 Editor 常驻槽（对齐 `c1760` 深挖 B：瞬时模式，可打印键/Esc 还焦）。
2. **Per-entry（或 per-block）fold 覆盖**：在全局默认之上，允许单块覆盖显隐；未覆盖块仍跟全局默认（迁移路径：第一次定点 toggle 写入覆盖表）。
3. **编号规则**：仅**当前视口可见**的可折叠头行；顺序 = 最新输出优先（底侧近输入为 1，或明确钉「距输入最近为 1」）；超过 10 个 → 只标最近 10，或分页（propose 钉）。
4. **键位**：默认保留 `alt+e` 进 leader；数字 `0`–`9`；Esc 退出。全局「全开/全关」若仍需要 → 另绑或 double-tap（Open；勿 silently 删掉无替代）。
5. **旁注/高亮**：编号叠在折叠标记旁；模式中旁注说明「按数字 toggle」。
6. **非目标**：鼠标点击（`c2040`）；Activity L2/L3（`c1760`/`c2050`）。

## Capabilities（意向）

- `app-tui-input` / `app-tui-transcript` — leader 模式与块级覆盖
- `app-tui-host` — 绘制编号层、paint-cache 失效规则
- 键位目录：新 action id（如 `app.tools.foldLeader`）vs 重载 `app.tools.blocks` — propose 拍板

## Impact

| 层 | 影响 |
|---|---|
| 产品 scrollback | 全局 bool → 全局默认 + 覆盖表；fingerprint 须含 per-entry 态 |
| 键位心智 | `Alt+E` 语义变化——跨面文档与 `c1760` 旁注须同步 |
| 性能 | 编号层 O(可见块)；进入/退出 MUST 局部 invalidate，禁止全历史 Markdown 重解析 |

## 依赖与排序

```text
[本 change c2030]  ← 无前置（Wave 0；可与 c2020 并行）
       │
       ├─blocks→ c2040（点击共用 per-block 覆盖表）
       └─blocks→ c2050（段级编号/目标模型）
```

- **无硬前置**；不依赖鼠标地基。
- **`c1760`**：无边；L2/L3 由 `c2050` 适配。和弦表（Alt+E vs Alt+Shift+E）propose 时与 `c1760` **文档对齐**，不作 apply 闸。
- 性能：`c1505` 非硬依赖。

## Out of scope

- 鼠标 / 三角字形美化（`c2040`）
- `activity.expandNearest` 栈（`c1760`）
- vim 通用 leader 引擎（景观缺口 #6；本波仅 fold 专用瞬时模式）

## Open Questions

1. `Alt+E` 完全改为 leader，还是 `Alt+E` 仍全局、`Alt+Shift+E` 留给 c1760、另开 leader 和弦？
2. Thinking（Ctrl+T）是否进同一编号平面，还是仅 tool/diff？
3. Compaction 块是否可编号 toggle（今日与 Alt+E 共用）？

## Ethics

- risk_level: medium（改默认和弦语义）
- prohibited_actions: 静默删除全局展开能力且无替代；编号模式吞掉普通数字输入却不退出；常驻抢 Editor 焦点
- required_evidence: harness：leader→digit 只改目标块；Esc 还焦；paint miss 上界
- escalation_policy: 默认和弦与 c1760 `Alt+Shift+E` 冲突方案须用户确认

## Further Notes

- 调研：[`research/fold-leader-vs-global-alt-e.md`](./research/fold-leader-vs-global-alt-e.md)
- 代码事实：`ScrollbackFold` 三全局 bool（`scrollback.rs`）；`app.tools.blocks` = `alt+e`
- 对齐：`c1760`「关键结构债：不是 per-entry」——本草案直接还这块债的 L1 部分
