---
brief: "写或改 AGENTS.md（抗衰老 / 墓碑描述清理 / 项目 skills 同审 / 文档放哪层 / 写法正反例）"
---

# AGENTS.md 编写与维护

真源归属：根 `AGENTS.md` 是**稳定操作边界**（全仓），各目录 `AGENTS.md` 只补本端例外；`llmanspec/AGENTS.md` 管 change/spec，`src/AGENTS.md` 管代码架构；文档落点见 `references/docs-loop.md`。本 reference 讲**怎么写、怎么抗衰老**。

## 只收三类内容

`AGENTS.md` = **稳定操作边界**，不是 README / 进度板。只收：

- **硬约束**
- **稳定原则**
- **宽泛索引（HOW 指针）**

API 名、调用序列、参数流水账等易腐细节一律以代码、skill 或 spec 为 SSOT，此处至多留一句指针——索引会过期，原则不会。

| 写 | 不写 |
|---|---|
| WHAT、硬约束、依赖方向、开闭规则 | 进度、行数快照、commit 列表、待办板 |
| HOW 指针（skill / 命令 / `llmanspec/specs`） | 长教程、完整测试矩阵、API / 调用序列枚举（代码为 SSOT）、易腐文件地图 |
| 与父级不同的本地例外 | 重复父级长文；复述他处 SSOT 已写全的事实（用指针替代）；钉死易变文件路径 / change id |

## 表达原则

写与改 `AGENTS.md`（及本 skill references）时的表达纪律：

| 原则 | 落法 |
|---|---|
| 加粗克制 | 只给「错了会坏数据 / 跨仓断裂」级规则加粗；情态词（必须 / 不得）不加粗 |
| 表格优先 | 并列清单、多成员集合、正反例用表，不用并列长句 |
| 图优先 | 结构 / 层级 / 生命周期 / 流程用 mermaid，不用文字铺陈拓扑 |
| 有重心 | 一条一个主题；一段混多事就拆条 |
| 数字克制 | 只保留守护基准（门禁阈值、硬版本下限）；易腐数字（行数、版本推导链）不进正文 |
| 引目录 | 指针落目录 / 层名 / crate 名，不钉具体文件（该文件本身是 SSOT 的除外） |
| 单一事实 | 同一事实只在一处写全，他处指针 |
| 无锁死信息 | 不写锁定 hash、构建机路径等难维护信息 |
| 常用字符 | 不用 emoji 与数学 / 特殊符号（集合符、双向箭头、省略号等），用词表达 |
| 无墓碑 | 移除即改写，不留「已删除 / 曾用 / 已退役」 |
| 不列脚本清单 | 脚本清单不进 AGENTS；首行模块注释即描述，需要时写一条注释提取命令 |

脚本清单提取命令（验证于本仓，`#` / `//` / `--` / Python docstring 头都识别；无首行注释的脚本自然不出现）：

```bash
rg --files -0 -g '*.{sh,py,lua,js,mjs,ts}' . | while IFS= read -r -d '' f; do awk 'NR==1&&/^#!/{next}/^[[:space:]]*$/{exit}/^[[:space:]]*(#|\/\/|--|""")/{sub(/^[[:space:]]*(#|\/\/|--|""")+[[:space:]]*/,"");sub(/("""|\/\/)+[[:space:]]*$/,"");h=h (h?" ":"") $0;next}{exit}END{if(h)print FILENAME "\t" h}' "$f"; done | sort
```

## 写法正反例速查

模式级对照，供直接套用；新增条目 MUST 长期为真（不钉具体文件名，勿随重构过时）：

| 场景 | 反例 | 正例 |
|---|---|---|
| 移除后描述 | 「`X` 已退役，原由 `Y` 接管」「曾用 A 实现」 | 只写现状：「A 由 `Y` 承担」；沿革查 git / archive |
| 字段 / 行为缺失 | 「`id` 字段作为死码已移除」 | 「幂等键只取 JSON-RPC `id` 的字符串形式」 |
| 接入步骤 | 枚举十几个方法名的调用链 | 一句 SSOT 指针（「完整序列以 `examples/foo.rs` 与导出文档为准」）+ 只留 MUST / 禁止 |
| 版本特定结论 | 「Cargo 1.97 下 incremental 会 stale」钉进正文 | 「具体行为随工具链版本变化，按当次版本验证」 |
| 易腐组成 / 步骤 | 内联枚举「fmt + clippy + 测试 + TUI harness」 | 短标签 + SSOT 指针：「全量门禁（组成见 justfile `qa`）」 |
| 大而稳定的集合 | 并列长句，增删各改多处 | 一张宽泛索引表（分层 / 命令 / 术语），只列稳定成员 |
| 进度 / 体量自述 | 「本模块 900 行，待拆」 | 软信号 `just complexity` / `just crap`；硬门禁见 `src/AGENTS.md` |
| 跨层重复 | 子目录复述父级依赖表 | 「依赖方向见 `src/AGENTS.md`；本端仅例外若干」 |
| 行为契约搬运 | 把 spec 场景抄进正文 | 「场景真源 `llmanspec/specs/<capability>/`」 |
| 版本段写进正文 | 「`v3` 映射」「`V4` 方法名」 | 「线协议映射」；协议版本只见 `PROTOCOL_VERSION` 与 `host.describe` 协商（版本段进标识符由命名闸卡住，见 `references/qa-gates.md`） |

## 偏好

全局规则与架构不变量；落点用层名（`agent`、`infra` 等），细节去读代码；易腐事实写「怎么查」（SSOT 指针），不写「现在是什么」。子目录只补本端例外。

**放哪**：全仓规则进根；目录边界进该目录 `AGENTS.md`；多步 how-to 进 skill（本仓团队 how-to 进 `xylitol-dev`）；临时笔记进 `_HANDOFF` / `*.tmp.md`（勿升格规范）；change 作用域调研进 `llmanspec/changes/<id>/research/`（细则 `llmanspec/AGENTS.md`）。

## 维护

- 六个月后是否仍真？否则不要进 AGENTS。
- **移除即改写**：不留「曾／已退役／原名」墓碑，描述对现状自解释（沿革查 git 与 archive）。
- **同审项目 skills**（`.agents/skills/` 下 `xylitol-*` 等）：事实性断言（路径 / 命令 / 约束）对照代码复核，漂移即修——skill 与 AGENTS 同一抗衰老纪律。
- **触发面自 harness**：`references/*.md` frontmatter `brief:` 用一句自解释的话说明本 reference 覆盖什么（它同时就是该文件的触发面）；SKILL.md 描述里 marker 之后的尾串由 `just gen-skill-description` 按文件名顺序把各 `brief` 用 `；` 拼成（漂移由 `scripts/check_skill_description.py` 随 `just qa` 卡住）。**新增 reference 必须写 `brief:` 并跑生成器**，勿手改尾串。
- 复杂度门禁与拆分默认策略在 `src/AGENTS.md`；**不**另维护超标表。
