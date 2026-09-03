# Design：c2510 探索簇近窗默认收起

## D1 形态抉择：簇头后缀（用户拍板），不做独立分组行

曾评估「独立 ✱ 分组行」形态：簇头 + 分组行并存，探索轮出现两行高度重叠的摘要
（簇头 `Explored 3 files` 与 ✱ 行 `✱ Explored — 5 reads · 2 searches`），且需要
新增行类型 / FoldTarget / 组 id 机制。用户拍板改为**扩展簇头本身**：

```
▸ Explored 3 files · 5 reads · 2 searches     ← 近窗默认（收起态）
```

关键红利：「簇头可见 + 子块不渲染」就是既有 L2 态（att26 在远窗已如此）——
本票只是把近窗默认地板对**纯探索簇**条件性降到 L2。展开（att28 就近 /
簇头命中）/ 命中区 / 渲染管线全部复用现成机制，零新增交互面。

## D2 触发判定：簇级纯度，不做游程切分

判定为纯函数：某簇的折叠中间段**全部** ∈ {读文件, 检索} 且总数 ≥ 阈值（默认
3）→ 探索簇。混入写类 / bash / compaction / ask / todo / MCP 任一段即不收起
（写类高价值，proposal 非目标；游程级局部收起没有展开手柄，簇级才 coherent）。
实现落点：summary.rs 的 `count_cluster` 已有类目统计，判定在其上薄封装。

## D3 可见性与状态

- 近窗（virgin window）地板：L0 → 探索簇 L2（`auto_collapse_explore=true` 时）；
  显式展开 = 既有 L2→L0 toggle（att28 / 簇头命中），进入既有显式级覆盖——
  新段到达 / 流式推进不重写显式级（复用 degrade 既有 explicit-state 保护）。
- 远窗 / rebuild：att26 与 `stream_collapse` 语义不变（探索簇同样被既有
  回合窗收纳，本票地板只在近窗生效）。
- 后缀只在「本策略收起态」渲染：用户展开后簇头回到 att24 基础词形（计数冗余）。
- att25 天然满足（L2 子块不进渲染）；ath25 行缓存无涉（不动已提交 UiEntry）。

## D4 双缝测试（已拍板）

- **纯函数缝（库内单测）**：簇判定边界——阈值 2 不收 / 3 收、混入写类或
  bash 不收、全读 / 读检索混合计数、`auto_collapse_explore=false` 恒不收。
- **harness 缝（BDD 可执行）**：HostPumpBdd 同族——ScriptedDriver 推
  ToolExecution 事件流，断言 scrollback 渲染行（默认仅簇头行 + 后缀、显式
  展开保持、混类反例、进行时 Exploring 计数更新）。与 designing 固定态
  一一对应。

## D5 designing 随动清单

`designing/tui/modules/activity-fold/`（原地扩展，不走 tui-lab）：

- intent.md：屏上词表增补「簇头计数后缀」条目（`· N reads · M searches`，
  进行时 Exploring）与 MUST（纯探索簇近窗默认收起、显式展开保持、混类不收）。
- states/：4 固定态——`explore-collapsed`（收起态簇头 + 后缀）/
  `explore-expanded`（显式展开，子块可见）/ `explore-running`（Exploring
  进行时计数）/ `mixed-not-collapsed`（混类不收反例）。
- draft.yaml：阈值与类别清单取舍点；chrome 词表 G 类增补后缀条目；
  `just gen-designing-index`（designing lint 入 qa；不动 token 不涉
  check-tui-tokens）。

## D6 配置键

`tui.activity_fold.auto_collapse_explore`（缺省 true，布尔；命名对齐
`auto_on_*` 家族）。specs 落 runtime-config 新增姊妹场景 rc29（已拍板），
既有 rc28 不动——避免 @human 锁定门禁的 rules_edit_acked。非法值（非布尔）
MUST 使配置加载失败。
