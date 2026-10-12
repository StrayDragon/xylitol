# Design — c2899 packages 子树情态词清整

## 映射表（对齐 c2896 已实施口径 + AGENTS 情态词表）

| 原文（英文情态） | 中文（按行型） |
|---|---|
| `MUST NOT` / `MUST NOT` 强调 | 描述行→`不得`；步骤行→`不` |
| `MUST` / `SHALL` | 描述行→`须`；步骤行→删（动词直述） |
| `SHOULD` | `宜` |
| `MAY` | 保留（建议词，非本票强制对像；出现处语义为「允许」已自然） |

- 作用面：`packages/xylitol-tui/llmanspec/specs/**.feature` 全部 18 capability；**只动规则描述行与场景步骤行**，`# 头注释`/`规则:`标题/场景名/标签行不变。
- 行型判定：场景块内 6 空格缩进 `假如/当/那么/并且` 开头 = 步骤行（约 10 处）；其余 4 空格缩进 = 描述行。脚本按缩进+前缀判型，人工 review 全量 diff。
- **BDD 同步**：步骤文本被 `tests/bdd/*bindings*`+`steps_*` 按精确文本绑定——凡改步骤行，grep 主 crate steps 引用并同 commit 修（预计极少：包场景多经 `#[scenario]` + 通用 steps；逐一验证 `cargo test --test bdd`）。
- 锁定规则：改 `规则:` 块描述 = WARNING 报告制，不阻断；`llman-sdd change diff` 留档。
- 验收：`grep -rE "MUST|SHALL|SHOULD" packages/.../specs` 仅剩引用原文/技术标识符（预期 0）；三根 validate 结构绿；BDD 全量绿；`just qa`。

## 非目标

- 不动 root `llmanspec/specs/**`（c2896/97 已清）。
- 不改语义：同义替换，句式重构仅当直译不通（逐处 diff review）。
- 不处理 `MAY`、不新增规则/场景。
