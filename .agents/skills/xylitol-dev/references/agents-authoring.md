# AGENTS.md 编写与维护

真源归属：根 `AGENTS.md` 是**稳定操作边界**（全仓），各目录 `AGENTS.md` 只补本端例外；`docs/AGENTS.md` 管产品文档闭环，`llmanspec/AGENTS.md` 管 change/spec，`src/AGENTS.md` 管代码架构。本 reference 讲**怎么写、怎么抗衰老**。

## 只收三类内容

`AGENTS.md` = **稳定操作边界**，不是 README / 进度板。只收：

- **硬约束**
- **稳定原则**
- **宽泛索引（HOW 指针）**

API 名、调用序列、参数流水账等易腐细节一律以代码、skill 或 spec 为 SSOT，此处至多留一句指针——索引会过期，原则不会。

| 写 | 不写 |
|---|---|
| WHAT、硬约束、依赖方向、开闭规则 | 进度、行数快照、commit 列表、待办板 |
| HOW 指针（skill / 命令 / `docs/architecture`） | 长教程、完整测试矩阵、API / 调用序列枚举（代码为 SSOT）、易腐文件地图 |
| 与父级不同的本地例外 | 重复父级长文；复述他处 SSOT 已写全的事实（用指针替代）；钉死易变文件路径 / change id |

## 写法正反例速查

模式级对照，供直接套用；新增条目 MUST 长期为真（不钉具体文件名，勿随重构过时）：

| 场景 | 反例 | 正例 |
|---|---|---|
| 移除后描述 | 「`X` 已退役，原由 `Y` 接管」「曾用 A 实现」 | 只写现状：「A 由 `Y` 承担」；沿革查 git / archive |
| 字段 / 行为缺失 | 「`id` 字段作为死码已移除」 | 「关联只走 `rpcId`；无传输级 `id` 字段」 |
| 接入步骤 | 枚举十几个方法名的调用链 | 一句 SSOT 指针（「完整序列以 `examples/foo.rs` 与导出文档为准」）+ 只留 MUST / 禁止 |
| 版本特定结论 | 「Cargo 1.97 下 incremental 会 stale」钉进正文 | 「具体行为随工具链版本变化，按当次版本验证」 |
| 易腐组成 / 步骤 | 内联枚举「fmt + clippy + 测试 + TUI harness + …」 | 短标签 + SSOT 指针：「全量门禁（组成见 justfile `qa`）」 |
| 大而稳定的集合 | 并列长句，增删各改多处 | 一张宽泛索引表（分层 / 命令 / 术语），只列稳定成员 |
| 进度 / 体量自述 | 「本模块 900 行，待拆」 | 软信号 `just complexity`；门禁见 `src/AGENTS.md` |
| 跨层重复 | 子目录复述父级依赖表 | 「依赖方向见 `src/AGENTS.md`；本端仅例外：…」 |
| 行为契约搬运 | 把 spec 场景抄进正文 | 「场景真源 `llmanspec/specs/<capability>/`」 |

## 偏好

全局规则与架构不变量；落点用层名（`agent`/`infra`/…），细节去读代码；易腐事实写「怎么查」（SSOT 指针），不写「现在是什么」。子目录只补本端例外。

**放哪**：全仓规则 → 根；目录边界 → 该目录 `AGENTS.md`；多步 how-to → skill（本仓团队 how-to 进 `xylitol-dev`）；临时笔记 → `_HANDOFF` / `*.tmp.md`（勿升格规范）；**change 作用域调研** → `llmanspec/changes/<id>/research/`（细则 `llmanspec/AGENTS.md`，勿堆 `docs/research/`）。

## 维护

- 六个月后是否仍真？否则不要进 AGENTS。
- **移除即改写**：不留「曾／已退役／原名」墓碑，描述对现状自解释（沿革查 git 与 archive）。
- **同审项目 skills**（`.agents/skills/` 下 `xylitol-*` 等）：事实性断言（路径 / 命令 / 约束）对照代码复核，漂移即修——skill 与 AGENTS 同一抗衰老纪律。
- 复杂度门禁与拆分默认策略在 `src/AGENTS.md`；**不**另维护超标表。
