# Design: c2920-refactor-compaction-trigger-scenario-outline

## 决策

### D1: 两个大纲而非一个

r1406 的 5 个场景实为两组不同前置形态：

- **reserve 闸组**（need-compact / no-compact / disabled-no-compact）：背景 window=100000（来自 feature `背景:`），变量仅 `tokens / enabled`，结果 `shouldCompact` 布尔。
- **floor 组**（floor-threshold-holds / floor-cross-triggers）：场景内显式覆盖 window=32768 且引入 `keepRecentTokens / 固定请求开销` 两个额外 given，变量 `tokens`（enabled 恒 true，背景已给）。

合并成单表会引入「部分列对部分行无意义」的稀疏矩阵（floor 组需 window/keep/overhead 列，reserve 组需 enabled 列），可读性反而下降。两个大纲各自小而完整，语义分组与规则句的论述顺序一致。

### D2: enabled 参数化走新 given 而非复用真值分支

既有 `compaction enabled 为 true` / `... 为 false` 是两条无参 given。大纲表内需要单步表达布尔，新增 `compaction enabled 为 {flag}`（`{flag}` 无类型提示 → `.+?` 捕获 → `.parse::<bool>()`，rstest-bdd 对 FromStr 类型的标准通路）。原两条 given 保留：`floor` 组之外还有场景（如 auto-over-threshold 组、c1640 系列）以文字形式使用它们，删了会牵连无关场景。

### D3: expect 列命名与 then 步骤

结果列在表内命名 `expect_compact`，then 步骤复用既有 `返回 true` / `返回 false`？——不行：大纲行内 then 必须消费占位符才能逐行断言。新增 then `返回 {expect_compact}`（parse bool 后与 `agent.compaction_result` 比对），与既有两条无参 then 并存（后者被 summarize 等场景使用）。

### D4: binding 签名

outline binding 函数参数名必须与 Examples 列名一致（rstest-bdd `find_matching_parameter` 按 ident==header 匹配，`_` 前缀可剥除）。`__rstest_bdd_case_idx` 由宏自动插入，手写签名不含它。fixture（`agent` / `ws`）照常声明。

## 权衡

- 备选「保持 5 场景不动」：零风险但重复矩阵依旧；本 change 正是验证大纲通路的最小试点，价值在通路本身。
- 表列名用英文（tokens/enabled/expect_compact）：与步骤内既有 typed capture（`{tokens:u32}`）命名一致，且 rstest-bdd header 匹配对 ident 是直接相等比较，中文列名虽可行但与既有 Rust 标识符风格断裂。
