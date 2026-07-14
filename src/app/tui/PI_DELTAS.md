# app/tui ↔ pi coding-agent 刻意差异台账

> **目的**：对照 `../pi/packages/coding-agent` interactive 做 UX 对齐时，**不得静默覆盖**本文件列出的 xylitol 产品决议。
> **定位**：本面是 xylitol 产品 TUI（host + Driver seam），不是 pi interactive 的 1:1 port。包层差异见 [`packages/xylitol-tui/PI_DELTAS.md`](../../../packages/xylitol-tui/PI_DELTAS.md)。
> **不是**进度板；能力差距清单见 `design/session-tree-vs-pi.md`。稳定边界见本目录 `AGENTS.md`。
> 新增刻意差异时：**先改代码与测试（或明确不实现），再在本表加一行**；回退差异须显式评审。

对齐源路径（历史参考）：`../pi/packages/coding-agent`（interactive / tree / travel）。

---

## 如何使用

1. 从 pi 拉行为或补丁前，先扫本表「不得回退」列。
2. 若 pi 变更触及某行主题，默认 **保留 xylitol 侧**；只有产品明确要求才改决议并更新本表。
3. 纯 bugfix（两边语义一致）可对齐 pi，不必记入本表。
4. 包组件行为差异记在 `packages/xylitol-tui/PI_DELTAS.md`，不要重复抄进本表。
5. 缺能力优先按 xylitol 产品需求设计；需要扩展点时先留 hook，再接具体策略。

---

## 刻意差异（normative）

| ID | 主题 | pi coding-agent | xylitol `src/app/tui` | 不得回退 |
|---|---|---|---|---|
| A01 | Travel 时分支摘要 | travel / 切分支时可走 LLM（或同类）生成 branch summary 写回树 | **不做** travel 时自动摘要；树节点文案来自 entry / label / 既有 summary 字段。若以后要策略，经 **hook / 扩展点** 注入，不内置默认 LLM 路径 | 是 |
| A02 | 同会话 fork 形态 | 树内 `/fork` 等可在同会话 MessageHistory 上开兄弟枝（再配合新会话文件语义） | 产品 **Shift+F / `/fork`** = `Driver::fork_session`（**新 child session** + switch）；同会话兄弟枝靠 **travel 改 leaf 后再发消息**（`parent_id`←当前 leaf）长出来。demo `agent_demo` 的同会话 Shift+F 是原型，**不是**产品默认语义 | 是 |

---

## 分叉可见性（手测备忘）

| 现象 | 原因 | 怎么看到分叉 |
|---|---|---|
| `/debug session-tree-multiturn` / `labeled` 打开是一条脊 | 线性夹具 | 用 **`/debug session-tree-branched`**，或见下「同会话兄弟枝」手测 |
| 一直 Fake 聊天、从不 travel | 每条消息挂在 tip leaf → 永远一条链 | 先 travel，再发 |
| Shift+F / `/fork` 后树仍像一条链 | 你已切到 **新 session**；父会话树不会自动出现「旁路子会话」节点 | 在父会话里 travel+续聊看兄弟；或分别打开父子 session 对比 |

**同会话兄弟枝（产品已有机制）**

1. 有多轮历史（手聊或 `/debug session-tree-multiturn`），**或** 直接 `/debug session-tree-branched`。
2. 若未用 branched：双 Esc / `/tree` → 选中**中部**节点 → Enter travel → 再发一条用户消息。
3. 开树：应能看到同一父节点下 **≥2 个孩子**（原枝 + 新枝）。fold / ←→ 分支跳转即可验。

**跨会话 fork（产品已有机制）**

- 树内 Shift+F，或叶上 `/fork` → `fork_session` + `switch_session`（user→`Before` / 非 user→`At`）。

`session-tree-branched` 预置兄弟枝；PTY：`pty_product_fake_session_tree_branched`。线性夹具测不出分叉 UI，不等于没有机制。

---

## 变更记录（短）

| 日期 | 变更 |
|---|---|
| 2026-07-14 | 建表；A01 明确不做 travel 分支摘要（撤 c695）；A02 产品 fork=新 session vs 同会话 travel 分枝 |
| 2026-07-14 | `/debug session-tree-branched` 预置兄弟枝；PTY `pty_product_fake_session_tree_branched`（raw 断言，避 CapturedScreen 长 scrollback 不同步） |
