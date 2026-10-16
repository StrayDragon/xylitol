---
brief: "docs 落点与产品↔实现闭环（roadmap 兑现迁移 / 术语表 / 黑话对照 / 内部编号语义尾 / 跨端同源）"
---

# 文档落点与产品↔实现闭环

**真源**：`docs/AGENTS.md`（docs 三层分工与闭环规则）、`llmanspec/AGENTS.md`（change/spec）、`src/AGENTS.md`（代码架构）。本文件只给**决策摘要 + 指针**，不复述全文。

## 一事实只写一处

| 层次 | 写什么 | 落点 |
|---|---|---|
| 产品**已落地**心智 | 现行 MUST / 禁止、开箱 vs 后置、多端同构 | `docs/architecture/` |
| 产品**未落地**方向 | 候选主线、依赖、BDD 意图（不维护状态列） | `docs/roadmaps/` |
| **行为合约** | 可验证 MUST/SHALL、BDD 场景 | `llmanspec/specs/` + `llmanspec/changes/` |
| **代码架构不变量** | 分层、seam、`Xy*`、复杂度 | `src/AGENTS.md` |
| 跨 change 耐久调研 | 主题级底稿（六个月后仍被引用） | `docs/research/` |

## 产品 ↔ 实现闭环

```text
docs/roadmaps/          →  认领方向、写清产品意图
        ↓
llmanspec/changes/…     →  提案 + specs（BDD）+ design/tasks
        ↓  apply / verify / archive
docs/architecture/      →  能力落地后，把「已成事实」的 MUST/禁止迁入
        ↓
docs/roadmaps/          →  删除已兑现段落；整篇兑现则删文件并更新索引
```

- 未落地的方向**禁止**提前写成 `docs/architecture/` 的现行 MUST。
- roadmap 整篇兑现 → **删文件**并更新索引，禁止长期占位 stub。
- change 临时调研落点：`llmanspec/changes/<id>/research/`（随 change 进 archive）；**禁止**为此新开或堆进 `docs/research/`。

## 何时动哪一层

| 情况 | 去做 |
|---|---|
| 只有产品方向、尚无行为合约 | 只改 `docs/roadmaps/` |
| 要改 MUST/SHALL 行为 | `llmanspec` SDD 全路径 |
| 行为已落地、文档仍只在 roadmap | 执行上方迁移 |
| 不改合约的小改 | 快速路径；产品文案若变仍更新 `docs/architecture/` |

## 术语表用法

黑话 ↔ 标准词 ↔ 英文对照总表：`docs/architecture/术语表.md`。

- 散文写作遇黑话**首现**时附标准词或英文对照（根 `AGENTS.md` 工作原则）。
- 内部编号（`c####` / `ath##`）单独引用时带 ≤12 字语义尾（如 `c2040 tui-mouse-click-fold-triangle`），避免检索死端。
- 表内只放**对照与一句话定义**；进度 / 状态列会腐化，不入表。

## 跨端公共体验

TUI 与第二产品端（gpui 桌面）共有能力按「一套学习成本」叙述；MUST / 禁止细则**唯一**约束板 `docs/roadmaps/跨端同源.md`。改公共交互前先按约束板对齐全部端；未开放端不要写成现行 MUST。
