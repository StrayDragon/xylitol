# docs/ 产品文档边界

本目录是**产品侧**叙事：今天是什么。
实现侧合约与变更在 `llmanspec/`；代码架构不变量在 `src/AGENTS.md`。
全局工作方式见根 `AGENTS.md`。

## 目录分工

| 路径 | 写什么 | 不写什么 |
|---|---|---|
| [`architecture/`](./architecture/AGENTS.md) | **已落地**产品心智：MUST/禁止、开箱 vs 后置、多端同构 | 易腐实现路径、进度板、未兑现方向、带日期的 change id |
| [`research/`](./research/) | **跨 change 仍常引用**的主题级耐久底稿 | 单 change 选型备忘、易腐深挖笔记（→ `llmanspec/changes/<id>/research/`，见 `llmanspec/AGENTS.md`）；未兑现产品方向 |
| 本文件 | docs 维护规则与产品↔实现闭环 | 具体能力正文（下沉到子目录） |

未兑现产品方向**不在本仓维护**：每个可感知能力一个 GitHub feature request；排序 / 依赖 / 认领切片集中在一个 [roadmap issue](https://github.com/StrayDragon/xylitol/issues/28)。

文风对齐：纯产品/领域语言（DDD），BDD 口吻描述意图；**尽量不写会漂移的代码细节**。

## 产品 ↔ 实现闭环（MUST）

无进行中的实现变更可认领时，默认按此环路推进（或复核）：

```text
GitHub feature request （+ roadmap 索引）
        ↓  认领切片、写清产品意图
llmanspec/changes/…     →  提案 + specs（BDD）+ design/tasks；关注实现侧可验证合约
        ↓  apply / verify / archive
docs/architecture/      →  能力落地后，把「已成事实」的产品 MUST/禁止迁入 architecture
        ↓
关闭对应 GitHub issue；roadmap 索引勾掉该切片
```

| 侧 | 关注点 |
|---|---|
| **docs（产品）** | 用户碰到什么、领域词汇、开箱/后置、跨端语义 |
| **llmanspec（实现）** | 可验证行为合约、变更原子性、BDD 场景真源 |
| **GitHub** | 未兑现方向的记忆与认领；不在仓内复写正文 |

### 跨端公共体验（写文档时）

已开放端（Print / TUI / Host）**共有**能力：产品文必须按「一套学习成本」叙述，语义同源、端专属才分叉——MUST / 禁止细则在 [`architecture/库与多客户端.md`](./architecture/库与多客户端.md)。未开放端（gpui 等）不要写成现行 MUST。

| 写哪 | 怎么写 |
|---|---|
| 未兑现公共交互 | GitHub feature request（gpui 半边 [#11](https://github.com/StrayDragon/xylitol/issues/11)）；可链 `llmanspec` 草案 |
| 已兑现公共心智 | `architecture/`（如多客户端） |
| 端专属（纯 TTY / 纯 GUI） | 可分叉；只在该端自己的文档讲 |

改公共交互相关 architecture / 各端 `AGENTS` 前，先按「产品规则」对齐全部已开放端。

### 兑现后迁移（MUST）

当某条 GitHub feature request（或其可交付切片）在产品上**已兑现**且对应 change 已归档（或等价已成为默认体验）时：

1. **写入 / 更新** `docs/architecture/` 中对应主题（新建或并入既有文），只保留稳定 MUST/禁止与用户心智；
2. **关闭**对应 GitHub issue；roadmap 索引勾掉该切片——禁止在仓内留未兑现正文或占位 stub；
3. architecture 可链仍候选的 GitHub issue（可选）；勿把未兑现方向写成现行 MUST。

### 何时动哪一层

| 情况 | 去做 |
|---|---|
| 只有产品方向、尚无行为合约 | 开或更新 GitHub feature request（roadmap 索引链过去） |
| 要改 MUST/SHALL 行为 | `llmanspec` SDD 全路径（explore → propose → …） |
| 行为已落地、文档仍只在 GitHub issue | **必须**执行上方迁移 |
| 小改不改合约 | 按根指南 quick 路径；产品文案若变，仍更新 architecture |

## 与其它 AGENTS 的关系

- 根 `AGENTS.md`：全仓工作原则；产品图入口指向本目录；含跨端公共体验一行指针。
- `src/app/tui/AGENTS.md`：TUI 端边界 + 跨端公共体验操作约束；固定区词表指针 → `architecture/TUI信息呈现与固定区词汇.md`。
- `llmanspec/AGENTS.md`：change/spec 命名与语言规则；含 change 附属 `research/` 落点。
- `src/AGENTS.md`：代码分层真值；**不**在 docs 重复分层长文。

## 维护习惯

- 先问：这条六个月后是否仍是**产品事实**？易腐进度 → 不进 AGENTS，不进 architecture 正文。
- 同一事实只在一处写全：现行 → architecture；未来 → GitHub issue；合约 → llmanspec。
- change 临时调研 / 选型深挖：落点规则见 `llmanspec/AGENTS.md`（勿堆 `docs/research/`）。
