---
depends_on: []
blocks:
  - c2040-add-tui-mouse-click-fold-triangle
  - c2050-update-activity-fold-mouse-leader
---

# TUI 折叠块 Leader + 数字键定点 Toggle

> **一句话**：`Alt+E` 进入折叠块编号模式（视口内近→远 1…9/0），数字只 toggle 目标块；全局 tools 改 `Alt+Shift+B`；保留 c1760 的 `Alt+Shift+E` 段栈。

## Why

今日 `Alt+E` 全局翻转所有 tool/diff（并连带 compaction）。无鼠标时无法定点折叠。本 change 提供 **键盘定点**（有鼠标后由 `c2040` 点击三角缓解，但仍保留 SSH / 关 capture 路径）。

## 已拍板（α′ · 2026-08-11）

| 项 | 决定 |
|---|---|
| 和弦方案 | **α′**（联合 c1760/c2040；见 research） |
| L1 leader | `Alt+E` → `app.tools.foldLeader`（重载原 `app.tools.blocks` 默认和弦） |
| L1 全局 tools | `Alt+Shift+B` → `app.tools.blocks`（或新 id `app.tools.blocksGlobal`；旁注跟绑定） |
| Compaction 全局 | **拆出** `Alt+Shift+C`（`app.compaction.toggle`）；不再与 Alt+E 同翻 |
| Thinking / 视口 | `Ctrl+T` / `Ctrl+O` **保持全局**；本波不做 leader |
| c1760 | **保留** `Alt+Shift+E` / `Ctrl+Alt+Shift+E`；本 change MUST NOT 占用 |
| 编号平面 | Tool / Diff / Ask（有稳定或合成目标键）；**不含** Thinking、Bash 块显隐 |
| 编号范围 | 进 leader 时扫**当前视口**，最多 10（`1`–`9`、`0`=第 10）；距输入最近为 1 |
| 取消 | Esc / 非映射可打印与编辑键 → 退出 mode；**MUST NOT 吞首字**（先清 mode 再投递或等价） |
| 全局 toggle 与覆盖 | 全局 tools toggle **改 default 并清空** per-block overrides（避免「按了没反应」） |
| 鼠标关系 | 本波无鼠标；与 c2040 **共用** per-block 覆盖表 / `fold.toggle` 语义 |

## What Changes

1. `FoldLeaderMode` 瞬时状态（映射 digit→目标）；落在 `UiRoot`（与今日 `fold` 同处）。
2. Per-block 覆盖表：`tools_expanded` 为默认；覆盖优先；fingerprint 含覆盖。
3. 键位目录与 DESIGN / `att7`/`att8` 旁注同步。
4. Paint：toggle 只从目标 entry 起失效；禁止进 leader 全历史 MD 重解析。

## Capabilities

- `app-tui-input` — leader / 全局 / compaction 键
- `app-tui-transcript` — 覆盖表、编号高亮、旁注和弦更新（修订 att7/att8）

## Impact

| 层 | 影响 |
|---|---|
| 键位 | Alt+E 语义变化；新增 Alt+Shift+B / Alt+Shift+C |
| scrollback | 全局 bool + overrides；ath25 miss 上界 |
| c1760 / c2040 | 文档对齐；覆盖表为点击前置 |

## 依赖与排序

Wave 0；无硬前置。`blocks` → c2040、c2050。与 c1760 无 `depends_on` 边，和弦表文档对齐。

## Out of scope

- 鼠标 / 字形（c2040）；Activity L2/L3（c1760/c2050）；vim 通用 leader；随滚动持续重标编号

## Open Questions

（无 — α′ 已关）

## 验证（自动化 + 人类）

| 层 | 自动化 | 人类 |
|---|---|---|
| Harness | Alt+E→编号；digit 只改目标；Esc/打字退出且首字入 Editor；Alt+Shift+B 全局+清覆盖；Alt+Shift+C 只翻 compaction | ≥3 tool → Alt+E → `2` 只动第二块 |
| Paint | 单块 toggle miss 上界；进 leader 无全表 MD 重解析 | 头行高亮、不整屏闪 |
| 键位 | 与 Alt+Shift+E（预留 c1760）不互吞；无默认 Alt+digit | 四终端冒烟可选 |

**人类最短路径**：造 ≥3 tool → `Alt+E` → 见编号 → `2` → Esc → 打字正常 → `Alt+Shift+B` 全局。

## Ethics

- risk_level: medium
- prohibited_actions: 静默删全局展开；吞首字；抢 Editor 常驻焦点；默认 Alt+digit；占用 c1760 的 Alt+Shift+E
- required_evidence: harness 定点/全局/还焦；paint miss 上界
- escalation_policy: 无（α′ 用户已确认）

## Further Notes

- [`research/fold-chord-with-activity-and-mouse.md`](./research/fold-chord-with-activity-and-mouse.md) — α′ 决议
- [`research/fold-axes-inventory-and-shift-global.md`](./research/fold-axes-inventory-and-shift-global.md)
- [`research/fold-leader-seams-code-facts.md`](./research/fold-leader-seams-code-facts.md)
- [`research/terminal-chord-alt-shift.md`](./research/terminal-chord-alt-shift.md)
- [`research/fold-leader-vs-global-alt-e.md`](./research/fold-leader-vs-global-alt-e.md)
