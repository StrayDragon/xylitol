# docs/ 产品文档边界

本目录是**产品侧**叙事：今天是什么、明天往哪走。
实现侧合约与变更在 `llmanspec/`；代码架构不变量在 `src/AGENTS.md`。
全局工作方式见根 `AGENTS.md`。

## 目录分工

| 路径 | 写什么 | 不写什么 |
|---|---|---|
| [`architecture/`](./architecture/README.md) | **已落地**产品心智：MUST/禁止、开箱 vs 后置、多面同构 | 易腐实现路径、进度板、未兑现方向、带日期的 change id |
| [`roadmaps/`](./roadmaps/README.md) | **未落地**统一候补：可并行主线、依赖、BDD 意图级场景（不维护状态列） | 可执行 `.feature`、模块/类型清单、进度勾选 |
| [`research/`](./research/) | **跨 change 仍常引用**的主题级耐久底稿 | 单 change 选型备忘、易腐深挖笔记（→ `llmanspec/changes/<id>/research/`，见 `llmanspec/AGENTS.md`） |
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
docs/roadmaps/          →  删除已兑现段落；整篇兑现则删文件并更新索引
```

| 侧 | 关注点 |
|---|---|
| **docs（产品）** | 用户碰到什么、领域词汇、开箱/后置、跨面语义 |
| **llmanspec（实现）** | 可验证行为合约、变更原子性、BDD 场景真源 |

### 跨面公共体验（写文档时）

TUI 与未来 Web 等**共有**能力：产品文必须按「一套学习成本」叙述——动作语义同源；快捷键 / 发现方式尽量同构（允许 OS 修饰键差异与面内增强如点击）。**仅**面专属能力才分开讲。

| 写哪 | 怎么写 |
|---|---|
| 未兑现公共交互 | `roadmaps/Web与TUI同源.md`（及衔接篇）；可链 `llmanspec` 草案 |
| 已兑现公共心智 | `architecture/`（如多客户端）；勿把未开闸面写成现行 MUST |
| 面专属（纯 TTY / 纯 DOM） | 可分叉；**禁止**把专属键位/流程写成公共 MUST |

改公共交互相关 roadmap / architecture / 面 `AGENTS` 前先对齐全套面，禁止静默开出「只教 TUI」或「只教 Web」的第二套公共故事。细则约束板：[`roadmaps/Web与TUI同源.md`](./roadmaps/Web与TUI同源.md)。

### ROADMAP 落地后迁移（MUST）

当某条 roadmap 主线（或其可交付切片）在产品上**已兑现**且对应 change 已归档（或等价已成为默认体验）时：

1. **写入 / 更新** `docs/architecture/` 中对应主题（新建或并入既有文），只保留稳定 MUST/禁止与用户心智；
2. **收敛** `docs/roadmaps/`：删掉已兑现段落；若整篇已无未兑现内容，**删除该文件**并更新索引——禁止长期占位 stub；
3. **交叉链接**：architecture 可链回仍候补的 roadmap（可选）；勿在 roadmap 里复述已迁入的正文。

未落地的方向**禁止**提前写成 architecture 的现行 MUST（可用「理想 vs 现状」区分的除外，且须标明未兑现）。

### 何时动哪一层

| 情况 | 去做 |
|---|---|
| 只有产品方向、尚无行为合约 | 只改 `roadmaps/`（或草稿提案，不挡他人 BDD） |
| 要改 MUST/SHALL 行为 | `llmanspec` SDD 全路径（explore → propose → …） |
| 行为已落地、文档仍只在 roadmap | **必须**执行上方迁移 |
| 小改不改合约 | 按根指南 quick 路径；产品文案若变，仍更新 architecture |

## 与其它 AGENTS 的关系

- 根 `AGENTS.md`：全仓工作原则；产品图入口指向本目录；含跨面公共体验一行指针。
- `src/app/tui/AGENTS.md`：TUI 面边界 + 跨面公共体验操作约束；chrome 词表指针 → `architecture/TUI信息面与chrome词汇.md`。
- `llmanspec/AGENTS.md`：change/spec 命名与语言规则；含 change 附属 `research/` 落点。
- `src/AGENTS.md`：代码分层真值；**不**在 docs 重复分层长文。

## 维护习惯

- 先问：这条六个月后是否仍是**产品事实或明确方向**？易腐进度 → 不进 AGENTS，不进 architecture 正文。
- 同一事实只在一处写全：现行 → architecture；未来 → roadmaps；合约 → llmanspec。
- change 临时调研 / 选型深挖 → **只**写 `llmanspec/changes/<id>/research/`；勿把易腐笔记塞进 `docs/research/`。
