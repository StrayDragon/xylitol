# Spec 过死：产品级 vs 代码组织

> Change `c2210`。2026-08-14。一手来源优先。

## 1. 本仓政策 vs 执行

`llmanspec/AGENTS.md`「spec 约束层级」：

- requirement MUST 描述产品可观察行为 / 数据契约；
- **禁止**硬约束路径、模块名、类型名、行数、方法归属、迁移清单；
- 例外：分层依赖、端口 seam、crate 边界、组合根、跨面同源；
- 代码组织演进不要求改 spec；过期主语可 **直接编辑** spec 免 change 生命周期。

执行：65 个 `spec.toon` 中 **54** 个含 `src/`。抽样：

| spec | 问题 |
|---|---|
| `layer-architecture` `la1` | 点名 `src/agent/`、`src/protocol/`、禁止 `src/domain/` |
| `app-tui-host` `ath12` | `host/mod.rs`、复杂度脚本、行数软顶 |
| `app-tui-chrome` `atc4` | 视觉 SSOT = `DESIGN.md` + `design/*.md` |
| `app-tui-design-playground` | HTML/lint/Agent-忽略；`valid_scope` 全是文件路径 |
| `package-tui-testing` `tt02` | `virtual_terminal_test.rs:178-184` |
| `cli-entry` `ce1` | `src/app/cli/` 模块位置 |
| `domain-security` `r71` | `src/agent/trust/` 删除清单 |

约 **1246** 条 requirement 量级。`app-tui-chrome` 单文件 26 条，含 footer 字段序与紧凑 k 规则——这些 **是** 可观察产品行为，应留，但不应再绑 `format_compact_tokens` 函数名（`atc24` 已点名）。

Partitioned 双写：可执行 GWT 只在 `.feature`；toon 里大量 `feature: false` 套话（「由单测覆盖」）。阅读差，且让人以为 spec 很厚。

## 2. 行业尺子

**OpenSpec Concepts**（[lzw.me/docs/openspec](https://lzw.me/docs/openspec/en/concepts.html) 对官方概念页的转载；原则与 Fission OpenSpec 一致）：

- Spec 是行为合约，不是实现计划。
- 属于 spec：可观察行为、输入输出与错误、外部约束、可测场景。
- **不属于**：内部 class/函数名、库选择、逐步实现、详细 execution plan（那些去 design/tasks）。
- 判定：**实现能改、对外行为不变 → 不进 spec。**

**Spec Kit**（GitHub specify）：`spec.md` = What/Why only；How 在 `plan.md`；工程纪律在 `constitution.md`。对照本仓：constitution ≈ 根/`src` `AGENTS.md`；plan ≈ change `design.md`；spec ≈ `llmanspec/specs`。把 constitution 写进 live spec，等于把 AGENTS 合法化成 BDD 闸，重构先改合约。

**OpenSpec vs Spec Kit**（[Big Hat 对照](https://www.bighatgroup.com/blog/openspec-vs-speckit-spec-driven-ai-development/)、[Avasdream](https://avasdream.com/blog/openspec-vs-spec-kit-ai-development)）：Spec Kit 偏绿场、阶段闸严；OpenSpec 偏棕场、delta、强调 spec 轻。xylitol 已是棕场 + 大量 live spec，更该 **减** 实现约束，而不是再加路径 MUST。

## 3. 推荐分层（本仓）

```text
用户可观察  ──►  llmanspec/specs     （少、稳、无歧义）
分层/缝/crate ──►  src/AGENTS.md      （大组织方向：spec 可留一句指针）
怎么做本次   ──►  change design/tasks
例子与回归   ──►  测试 / .feature     （不要把测试路径写进 toon）
```

**Keep 在 spec 的例子：** idle 时 status 0 行；簇头 Used 按调用次数；Trust 闸的是项目本地资源。

**Move 到 AGENTS 的例子：** ath12 的文件切分与复杂度数字；la1 的目录点名（改成「agent 编排、protocol 契约、禁止第三顶栏」不写路径）；adp 整 capability 若 playground 改为生成物则删或改成「产品视觉目录由场景生成、lint 进 qa」一句。

**Delete：** 已删除类型的防复活；过期 Next-wave 槽名单（`adp10` 仍写 c491 时代的「下一波」）。

## 4. 歧义闸

c1762 类失败：spec/design 写了词表，没钉「N=次数还是去重」「Thought 是簇头还是 L1」。新 req 准入：

1. 主语是用户或下游系统能看见的东西；
2. 有反例（明确 MUST NOT）；
3. 计数/时态/身份有定义；
4. 不提文件路径除非 crate 边界；
5. 作者能说出一个会误解的读法并已经排除。

做不到 → 留在 proposal，不进 live spec。

## 5. 派工形状

审计表列：`capability, req_id, 现行 statement 摘句, 判定, 改写草稿`。先 TUI+layer+playground+testing，再全量。改 live toon 在绑定分支；组织过期可走「直接编辑 spec」条款，仍建议 PR 可审。
