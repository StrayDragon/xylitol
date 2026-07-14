---
version: "alpha"
name: "session-tree"
description: "Double-Esc session tree — demo SSOT; product c615 live MessageHistory."
tokens_from: "../DESIGN.md"
components:
  tree-line:
    textColor: "{colors.on-surface}"
  tree-muted:
    textColor: "{colors.muted}"
  tree-kind-user:
    textColor: "{colors.user}"
  tree-kind-assistant:
    textColor: "{colors.success}"
  tree-kind-tool:
    textColor: "{colors.tool}"
---

# Session tree

> Token 根源：`{colors.*}` → [`../DESIGN.md`](../DESIGN.md)。

**优先路径**（替代 Codex 式 transcript 浏览）：双 Esc 打开会话树。
**形态学 SSOT**：`packages/xylitol-tui` `agent_demo`（活树 / travel / filter / kind…）。
**产品 c615**：双 Esc → `Driver::session_tree(MessageHistory)` 活树；Enter → `travel_session_tree`（user → `editor_text` 预填 + leaf 更新 + scrollback 按 ancestry 重建）。
**产品下一波**：filter **c635** → fold **c640** → fork **c645**（demo 已有；键位见 [`keybindings.md`](./keybindings.md)；静图 playground「Tree power」）。

## MUST

1. **空编辑器**双 Esc（时间窗与 demo 一致）打开会话树；**MUST NOT** 做成 Codex 式独立 transcript 浏览面。
2. 树 **替换 editor 槽**（`showSelector`），保证贴底可见；**MUST NOT** blit 到内容绝对顶部。
3. 选中用 **整行 reverse**（SGR `\x1b[7m` / 静图 `.rev`）；勿另立 selection 面板底色。
   - **未选中**：kind 前缀可用主题色（`user` / `assistant` / `tool`）。
   - **选中行**：reverse 独占对比度；kind **MUST NOT** 再叠独立前景色（否则紫/绿压在 reverse 底上对比度崩）。静图：`.rev` 内禁止有效的 `fg-*` 色穿透。
4. Esc：label 编辑中取消编辑 → 有搜索串清搜索 → 否则关闭树；流中单 Esc 仍为 abort；**忙碌时 MUST NOT 开树**。
5. travel / fork 经应用面 `Driver`；包组件只负责树 UI。demo **Enter travel（c600，对齐 pi）**：
   - `kind=user` → history leaf = **父节点**；user 正文预填 editor；transcript = root→父（**不含**被选 user 及其后线性回复）。
   - 非 user → leaf = 选中 id；重建 root→选中；**不**因 travel 预填 user 正文。
   - Shift+F fork：demo `ast5`（leaf=选中；user 预填）；产品 **c645**。
   - **产品（c615）**：Enter MUST 调 `travel_session_tree`；`editor_text` 有值时预填；scrollback 按 travel `leaf_id` ancestry 最佳努力重建。
6. **搜索**：对 label / kind / annotation 增量过滤（与 `include_node` AND）— demo 已有；产品随 **c635**。
7. **翻页**：←→ 与 PgUp/PgDn 按 `max_visible` 翻页。
8. **Filter**（产品 **c635**，对齐 pi）：Ctrl+D → default；Ctrl+T/U/L/A **toggle** ↔ default；Ctrl+O cycle；default 藏 `kind=meta` bookkeeping；状态行 `(i/n)` + 非 default 时 `[filter]`（无 `[default]`）。demo 曾用纯 set，产品以 pi toggle 为准。
9. **Fold / 分支跳转**：Ctrl/Alt+←→；连接符 ⊞/⊟（c467）— 产品 **c640**。
10. **Annotation**：可选 `[annotation]` + Shift+L 编辑 + Shift+T 时间戳（c467）— 可随 **c640** 或拆 future。
11. **Kind（c595）**：`TreeNode.kind` 为可选字符串；渲染序 `[annotation]?` + 主题化 kind 前缀 + **纯正文** `label`。**MUST NOT** 把 `user:` / `assistant:` / `tool:` 预烘焙进 `label` 作为唯一表现。

## 实现指针

| 层 | Change / 状态 |
|---|---|
| 包 `TreeSelector` | c454 + c456 + c467 + c469 pan + **c595 kind** |
| demo 活树 / travel | c469；steer 队列 c468；**c600** travel 对齐 pi |
| 产品 `src/app/tui` | **c615** 活树 + `effects::drain_pending` 异步泵 |
| 相对 pi 差距 | [`session-tree-vs-pi.md`](./session-tree-vs-pi.md) |
