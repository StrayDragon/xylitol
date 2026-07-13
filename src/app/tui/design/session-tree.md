---
version: "alpha"
name: "session-tree"
description: "Double-Esc session tree — demo SSOT; product c491 stub frozen."
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
**产品 c491**：**stub 冻结** — 仅假树槽替换 + `travel → id`；**MUST NOT** 在 stub 上扩展；真活树另 change（Track B，非本 stub）。

## MUST

1. **空编辑器**双 Esc（时间窗与 demo 一致）打开会话树；**MUST NOT** 做成 Codex 式独立 transcript 浏览面。
2. 树 **替换 editor 槽**（`showSelector`），保证贴底可见；**MUST NOT** blit 到内容绝对顶部。
3. 选中用 **reverse**；勿另立 selection 面板底色。
4. Esc：label 编辑中取消编辑 → 有搜索串清搜索 → 否则关闭树；流中单 Esc 仍为 abort。
5. travel / fork 经应用面 `Driver`；包组件只负责树 UI。demo **Enter travel（c600，对齐 pi）**：
   - `kind=user` → history leaf = **父节点**；user 正文预填 editor；transcript = root→父（**不含**被选 user 及其后线性回复）。
   - 非 user → leaf = 选中 id；重建 root→选中；**不**因 travel 预填 user 正文。
   - Shift+F fork 仍见 `ast5`（leaf=选中；user 预填）。
6. **搜索**：对 label / kind / annotation 增量过滤（与 `include_node` AND）。
7. **翻页**：←→ 与 PgUp/PgDn 按 `max_visible` 翻页。
8. **Filter**（demo/产品谓词）：Ctrl+D/T/U/L/A；树开时 Ctrl+O 循环；状态行 `(i/n) [filter]`。
9. **Fold / 分支跳转**：Ctrl/Alt+←→；连接符 ⊞/⊟（c467）。
10. **Annotation**：可选 `[annotation]` + Shift+L 编辑 + Shift+T 时间戳（c467）。
11. **Kind（c595）**：`TreeNode.kind` 为可选字符串；渲染序 `[annotation]?` + 主题化 kind 前缀 + **纯正文** `label`。**MUST NOT** 把 `user:` / `assistant:` / `tool:` 预烘焙进 `label` 作为唯一表现。demo 约定 `user` / `assistant` / `tool`；包 **MUST NOT** 硬编码产品 role 枚举。

## 实现指针

| 层 | Change / 状态 |
|---|---|
| 包 `TreeSelector` | c454 + c456 + c467 + c469 pan + **c595 kind** |
| demo 活树 / travel | c469；steer 队列 c468；**c600** travel 对齐 pi |
| 产品 `src/app/tui` | **c491 stub 冻结**（假树 + travel 行）；真 session/Driver travel **未接线** |
| 相对 pi 差距 | [`session-tree-vs-pi.md`](./session-tree-vs-pi.md) |
