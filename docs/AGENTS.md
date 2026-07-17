# docs/ 产品文档边界

本目录是**产品侧**叙事：今天是什么、明天往哪走。
实现侧合约与变更在 `llmanspec/`；代码架构不变量在 `src/AGENTS.md`。
全局工作方式见根 `AGENTS.md`。

## 目录分工

| 路径 | 写什么 | 不写什么 |
|---|---|---|
| [`architecture/`](./architecture/README.md) | **已落地**产品心智：MUST/禁止、开箱 vs 后置、多面同构 | 易腐实现路径、进度板、未兑现方向 |
| [`roadmaps/`](./roadmaps/README.md) | **未落地**方向板：可并行主线、依赖、BDD 意图级场景 | 可执行 `.feature`、模块/类型清单 |
| 本文件 | docs 维护规则与产品↔实现闭环 | 具体能力正文（下沉到子目录） |

文风对齐：纯产品/领域语言（DDD），BDD 口吻描述意图；**尽量不写会漂移的代码细节**。

## 产品 ↔ 实现闭环（MUST）

无进行中的实现变更可认领时，默认按此环路推进（或复核）：

```text
docs/roadmaps/          →  认领方向、写清产品意图
        ↓
llmanspec/changes/…     →  提案 + specs（BDD）+ design/tasks；关注实现侧可验证合约
        ↓  apply / verify / archive
docs/architecture/      →  能力落地后，把「已成事实」的产品 MUST/禁止迁入 architecture
        ↓
docs/roadmaps/          →  收缩或删除已兑现叙事，避免双源
```

| 侧 | 关注点 |
|---|---|
| **docs（产品）** | 用户碰到什么、领域词汇、开箱/后置、跨面语义 |
| **llmanspec（实现）** | 可验证行为合约、变更原子性、BDD 场景真源 |

### ROADMAP 落地后迁移（MUST）

当某条 roadmap 主线（或其可交付切片）在产品上**已兑现**且对应 change 已归档（或等价已成为默认体验）时：

1. **写入 / 更新** `docs/architecture/` 中对应主题（新建或并入既有文），只保留稳定 MUST/禁止与用户心智；
2. **收敛** `docs/roadmaps/`：删掉已兑现段落，或改成「已迁入 architecture」的短指针，禁止长期双份正文；
3. **交叉链接**：architecture 可链回历史意图（可选）；roadmap 索引表更新状态。

未落地的方向**禁止**提前写成 architecture 的现行 MUST（可用「理想 vs 现状」区分的除外，且须标明未兑现）。

### 何时动哪一层

| 情况 | 去做 |
|---|---|
| 只有产品方向、尚无行为合约 | 只改 `roadmaps/`（或草稿提案，不挡他人 BDD） |
| 要改 MUST/SHALL 行为 | `llmanspec` SDD 全路径（explore → propose → …） |
| 行为已落地、文档仍只在 roadmap | **必须**执行上方迁移 |
| 小改不改合约 | 按根指南 quick 路径；产品文案若变，仍更新 architecture |

## 与其它 AGENTS 的关系

- 根 `AGENTS.md`：全仓工作原则；产品图入口指向本目录。
- `llmanspec/AGENTS.md`：change/spec 命名与语言规则。
- `src/AGENTS.md`：代码分层真值；**不**在 docs 重复分层长文。

## 维护习惯

- 先问：这条六个月后是否仍是**产品事实或明确方向**？易腐进度 → 不进 AGENTS，不进 architecture 正文。
- 同一事实只在一处写全：现行 → architecture；未来 → roadmaps；合约 → llmanspec。
