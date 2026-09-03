# Design：c2510 探索簇头类目计数后缀

## D1 形态演进（三次收敛，最终形态用户拍板）

1. delayed 原案「独立 ✱ 分组行」：以 2026-08-23 的「近窗子块逐条平铺」痛点为
   前提——**该前提已被代码推翻**（`cluster_is_expanded`：L0/L2 均需
   `cluster_open`，live 窗同样；所有簇默认仅簇头行）。
2. 「扩展簇头 + 近窗条件收起」：收起已是普遍现状，条件地板与
   `auto_collapse_explore` 开关没有可门控的行为增量，随之删除（rc29 撤销）。
3. **最终形态：纯簇头信息密度增强**——`Explored 3 files` →
   `Explored 3 files · 5 reads · 2 searches`。

## D2 计数维度

att24 的 `Explored N files` 是**去重路径**计数；后缀是**调用次数**计数
（`ExploreKind::File` → reads，`ExploreKind::Search` → searches；无路径搜索
计入 searches）。两者可区分且同时为真：3 files · 5 reads = 五次读调用触达
三个不同文件。实现：`counts_from_atoms` 的 Explore 臂加计次；`search_no_path`
的调用同样计次进 searches。

## D3 后缀渲染规则

- 附着于探索分句：`Explored 3 files · 5 reads · 2 searches`（`·` 分隔）；
  Edited 头（文件层互斥）不附探索计数——att24 互斥精神的延伸，混类簇的
  探索信息可经展开子簇细账获得。
- 仅列非零类目；单数/复数随计数（1 read / 2 reads）。
- progressive（Exploring）与封口（Explored）同构携带；计数随工具开始更新
  是既有行为（ToolStart 即建条目、逐帧重算 counts），后缀自动继承。

## D4 双缝测试（已拍板）

- **纯函数缝（库内单测）**：`format_cluster_body` 后缀——多类目、单类目、
  无路径搜索计数、progressive 词形、Edited 头不附、omits 头不受影响。
- **harness 缝（BDD 可执行）**：`SceneBuilder` 回放 XyEvent → `render_plain`
  无头帧断言（att24 `cluster-head-wording-exclusivity` 同缝）：多读多检索封口
  帧、单类目帧、流式计数更新帧。

## D5 designing 随动清单

`designing/tui/modules/activity-fold/`（原地扩展）：intent.md 屏上词表增补
后缀条目（`· N reads · M searches`，进行时 Exploring 同构）；states 增
`explore-head-suffix`（封口收起态）与 `explore-head-suffix-running`（进行时）
固定态；draft.yaml 记录「只列非零类目 / Edited 不附」取舍；chrome 词表 G 类
同步一条；`just gen-designing-index`（designing lint 入 qa；不动 token 不涉
check-tui-tokens）。
