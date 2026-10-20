---
brief: "文档落点与产品↔实现闭环（GitHub FR/roadmap → SDD → specs / AGENTS / 黑话对照 / 内部编号语义尾 / 跨端同源）"
---

# 文档落点与产品↔实现闭环

**真源**：`llmanspec/AGENTS.md`（change/spec）、`src/AGENTS.md`（代码架构）、根 `AGENTS.md`（全仓操作边界）。本文件只给**决策摘要 + 指针**，不复述全文。

## 一事实只写一处

| 层次 | 写什么 | 落点 |
|---|---|---|
| 产品**已落地**行为 | 可验证 MUST / 禁止、开箱 vs 后置、多端同构 | `llmanspec/specs/<capability>/` |
| 产品**未落地**方向 | 可感知能力各一个 FR；排序 / 依赖单独一个 roadmap | GitHub Issues（索引 [#28](https://github.com/StrayDragon/xylitol/issues/28)） |
| **行为合约**（同上） | BDD 场景、`@req:` 句柄 | `llmanspec/specs/` + `llmanspec/changes/` |
| **代码架构不变量** | 分层、seam、`Xy*`、复杂度 | `src/AGENTS.md` |
| change 作用域调研 | 选型对照、易腐深挖 | `llmanspec/changes/<id>/research/`（随 change 进 `archive/`） |

## 产品 ↔ 实现闭环

```text
GitHub feature request （+ roadmap 索引）
        ↓
llmanspec/changes/…     →  提案 + specs（BDD）+ design/tasks
        ↓  apply / verify / finalize
llmanspec/specs/        →  能力落地后的 live 行为合约
        ↓
关闭对应 GitHub issue；roadmap 索引勾掉该切片
```

- 未落地的方向**禁止**提前写成 live `.feature` 的现行 MUST。
- 兑现后关 issue，禁止在仓内留未兑现正文或占位 stub。
- change 临时调研**只**进 `llmanspec/changes/<id>/research/`；关键结论摘要回写 `proposal.md`。

## 何时动哪一层

| 情况 | 去做 |
|---|---|
| 只有产品方向、尚无行为合约 | 开或更新 GitHub feature request |
| 要改 MUST/SHALL 行为 | `llmanspec` SDD 全路径 |
| 行为已落地、合约仍只在 issue | 执行上方迁移（landing specs + 关 issue） |
| 不改合约的小改 | 快速路径；若用户可见文案变，同步相关 spec 或 AGENTS 指针 |
| 代码组织演进导致 spec 过期 | 直接编辑 live `.feature`（见 `llmanspec/AGENTS.md`「spec 维护」） |

## 术语与写作

- 散文写作遇黑话**首现**时附标准词或英文对照（根 `AGENTS.md` 工作原则）。
- `规则:` 标题用标准词（`llmanspec/AGENTS.md`「语言」）。
- 内部编号（`c####` / `ath##`）单独引用时带 ≤12 字语义尾，避免检索死端。

## 跨端公共体验

已开放端（Print / TUI / Host）共有能力按「一套学习成本」叙述；MUST / 禁止细则在 `llmanspec/specs/protocol-app/` 与 `user-experience/`。改公共交互前先对齐全部已开放端；未开放端不要写成现行 MUST。gpui 半边见 [#11](https://github.com/StrayDragon/xylitol/issues/11)。
