# 探索分组 调研补充（c2510）

> 2026-08-23。结论先行：折叠的**机械层**（segment/state/summary/degrade）
> 已完备，缺的是**分类策略**——本票只加策略，不动机制与落盘合约。

## 现状核对（代码事实）

- activity_fold 模块构成：`mod.rs` / `atom.rs`（原子段）/ `segment.rs`
  （段）/ `scene.rs` / `state.rs` / `summary.rs` / `degrade.rs` / `settings.rs`
  （用户设置）/ `live_tape.rs`。
- 手动折叠入口：就近展开/收起（`app.activity.expandNearest`=Alt+Shift+E、
  `collapseNearest`=Ctrl+Alt+Shift+E）；块级折叠另有 expandable
  （thinking/tool/diff/ask 三角点击，c2040/c2045）。
- designing `activity-fold` 模块三态：collapsed / envelope / expanded——
  「信封」即既有摘要形态，探索分组的摘要行应复用同一视觉语言。

## 参照手法

成熟实现的自动分组要点：

1. 只合并**相邻同类低价值段**（连续 read/grep/glob），运行中显示进行时文案；
2. 合并是显示层行为：任一段被手动展开即退出该组（用户意志 > 自动策略）；
3. 阈值保守（≥3 段才合并），避免「什么都看不见」的反效果。

## 引入设计（规格草案）

- 分类器输入 = 既有 segment 类别序列，输出 = 分组决策
  （纯函数，可直测）；类别清单首版仅 {读文件, 检索}。
- 分组行文案走固定区词表新增条目（如 `Explored —` 前缀），需同步词表文档。
- 与 settings 打通：`ActivityFoldSettings` 增加 auto-group 开关（默认 on）。

## 决策点

1. 类别清单是否首期就含 glob/webfetch（建议不含，先验证手感）；
2. 分组行是否计入「就近折叠」的作用范围（建议是）。

## 影响面

- activity_fold 策略 + settings 字段；scrollback 渲染；harness 切片
  （合并/不合并/手动展开退组三例）；词表文档一条。
