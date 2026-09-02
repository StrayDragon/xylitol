# Design：c2510 探索分组

## D1 与既有折叠族的组合语义（核心权衡）

分组是**簇内子块层的默认聚合显示**，不是第四层结构：att23 的信封⊃簇⊃块嵌套、
att24 簇头词形、att26 回合窗收纳全部保持不变。可见性三层各管一段：

| 层 | 管什么 | 时机 |
|---|---|---|
| att26 信封折叠 | 整轮回收 | 回合末 / rebuild，keep_recent_turns 窗外 |
| att24 簇头 | 一簇活动的文件计数摘要 | 簇存在即渲染（子块默认展开） |
| **本票分组行** | 相邻同类检索子块的类目计数聚合 | **近窗/流式即生效**，≥3 才合 |

词形区隔：簇头 `Explored 3 files`（文件计数，att24 域）vs 分组行
`✱ Explored — 3 reads · 2 searches`（类目计数，`✱` 标记 + em-dash 格式，
本票新增词表条目，进 chrome 词表 G 类与 activity-fold intent 屏上词表）。
可见性合成与 att25 同构：组折叠时子块不进渲染，不触发 ath25 全量 Markdown
重解析。

## D2 用户意志优先的落地形态

分组展开态走 per-group 稳定 id 的覆盖语义（与 att20 per-block 覆盖、att23
稳定 id 同构）：手动展开某组后该组保持展开，不因组内新段到达或流式推进被自动
收起；组 id 在 live 与 travel/fork/resume 重建间同构。组内**新增**同类段继续
计入计数（摘要行计数增长），但不改变已展开组的展开态。

## D3 双缝测试（已拍板）

- **纯函数缝（库内单测）**：分类器输入 = segment 类别序列（含进行时状态），
  输出 = 分组决策（组边界、类别计数、进行时词形数据）。边界：阈值 2 不合 /
  3 合、类别断开即断组、写类/bash 混入即断、跨助手正文必断、auto_group=false
  恒不分。
- **harness 缝（BDD 可执行）**：HostPumpBdd 同族——ScriptedDriver 推
  ToolExecution 事件流，断言 scrollback 渲染行（默认合并行数、展开退组保持、
  进行时文案、写类相邻反例）。与 designing 四固定态一一对应。

## D4 designing 随动清单

`designing/tui/modules/activity-fold/`（用户已拍板原地扩展，不走 tui-lab）：

- intent.md：屏上词表增补分组行条目；MUST——默认合并（≥3）、手动展开退组、
  写类/bash/diff/compaction/ask 永不入组、组行词形（✱ + 类目计数 + em-dash）。
- states/：新增 4 固定态——`grouped-default`（合并行）/ `group-expanded`
  （手动退组保持）/ `group-running`（Exploring 进行时计数）/ `group-broken`
  （写类相邻不合并反例）。
- draft.yaml：阈值与类别清单取舍点（todos）。
- chrome 词表 `docs/architecture/TUI信息面与chrome词汇.md` G 类增补条目；
  `just gen-designing-index`（designing lint 入 qa；不动 token 不涉
  check-tui-tokens）。

## D5 配置键

`tui.activity_fold.auto_group`（缺省 true，布尔）。specs 落 runtime-config
**新增姊妹场景** rc29（已拍板），既有 rc28 不动——避免 @human 锁定门禁的
rules_edit_acked。非法值（非布尔）MUST 使配置加载失败，与 rc28 既有非法值
语义同构。
