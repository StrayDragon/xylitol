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
---

# Session tree

> Token 根源：`{colors.*}` → [`../DESIGN.md`](../DESIGN.md)。

**优先路径**（替代 Codex 式 transcript 浏览）：双 Esc 打开会话树。
**形态学 SSOT**：`packages/xylitol-tui` `agent_demo`（活树 / travel 回复链 / filter…）。
**产品 c491**：**stub 冻结** — 仅假树槽替换 + `travel → id`；**MUST NOT** 在 stub 上扩展；真图等开闸。

## MUST

1. **空编辑器**双 Esc（时间窗与 demo 一致）打开会话树；**MUST NOT** 做成 Codex 式独立 transcript 浏览面。
2. 树 **替换 editor 槽**（`showSelector`），保证贴底可见；**MUST NOT** blit 到内容绝对顶部。
3. 选中用 **reverse**；勿另立 selection 面板底色。
4. Esc：label 编辑中取消编辑 → 有搜索串清搜索 → 否则关闭树；流中单 Esc 仍为 abort。
5. travel / fork 经应用面 `Driver`；包组件只负责树 UI。demo：`agent_demo::travel_to_history` 按 root→id（+ 线性回复链）重建 transcript。
6. **搜索**：对 label/annotation 增量过滤（与 `include_node` AND）。
7. **翻页**：←→ 与 PgUp/PgDn 按 `max_visible` 翻页。
8. **Filter**（demo/产品谓词）：Ctrl+D/T/U/L/A；树开时 Ctrl+O 循环；状态行 `(i/n) [filter]`。
9. **Fold / 分支跳转**：Ctrl/Alt+←→；连接符 ⊞/⊟（c467）。
10. **Annotation**：可选 `[annotation]` + Shift+L 编辑 + Shift+T 时间戳（c467）。

## 实现指针

| 层 | Change / 状态 |
|---|---|
| 包 `TreeSelector` | c454 + c456 + c467 + c469 pan 已归档 |
| demo 活树 / travel | c469；steer 队列 c468 |
| 产品 `src/app/tui` | **c491 stub 冻结**（假树 + travel 行）；真 session/Driver **未开闸** |
| 相对 pi 差距 | [`session-tree-vs-pi.md`](./session-tree-vs-pi.md) |
