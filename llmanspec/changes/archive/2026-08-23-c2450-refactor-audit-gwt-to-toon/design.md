# Design

## 映射规则（沿 batch1/c2415）

| .feature 元素 | toon 行 |
|---|---|
| `@req:X` | req_id 列 = X |
| `场景: english-id` | id 列 = english-id（保留） |
| `假如 …` / 缺省 | given 列（缺省空串） |
| `当 …` | when 列 |
| `那么 …` | then 列 |
| 终列 | `false` |

- 引号规则：含空格/逗号/冒号/方括号的单元格加双引号；本批三个 `.feature` 全文零双引号，
  无转义需求。
- 本批三文件已验证：无 `并且` 链、无 docstring、无表格、无 `规则:` 块——纯单步 GWT。
- 与既有 toon 行 id 碰撞检查：input/host/chrome 均无碰撞。
- 三个 toon 均已有带列头的 `scenarios[N]{req_id,id,given,when,then,feature}:`，
  只需追加行并把 N 改为 N+新增数；不涉及 `scenarios[0]:` 特例。

## 已知特例

- 场景句式多为审计陈述（如「当 审计产品 TUI 源码 / 那么 无 TUI::start」），整段照搬 toon
  文档行，不改写为可执行步骤。
- input.feature 内 `alt-enter-followup`、`busy-enter-steer` 同名场景各出现两次
  （req 不同）；toon 行按 req 分列保留两份，id 冲突在删除 `.feature` 后消解。
