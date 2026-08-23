# 宽度断点档位 调研补充（c2520）

> 2026-08-23。结论先行：纵向预算已有体系（layout 槽高按行数），
> **横向没有档位表**——各组件的降级行为散落且口径不一。

## 现状核对（代码事实）

| 组件 | 现有横向处理 | 出处 |
|---|---|---|
| models picker | `to_select_item(focused, width_budget)` 显式接收宽度预算 | `layout/models_picker.rs:83` |
| footer | 字段序 `cwd · model · {thinking}` + 可续 token 用量，紧凑格式（如 `42k`）；窄宽截断 | designing `footer` 模块 / `layout/theme.rs` 一带 |
| atoms | 截断统一 `…` 规则 | designing `atoms` 模块 / 包层 utils |
| editor | 占位/边框随宽度自适应（editor-placeholder 为 lab 待定项） | `components/editor/layout.rs` |
| transcript/tool 行 | gutter + 内容缩进固定，窄宽行为未成文 | designing `tool` / `transcript` |

- 布局根只做**纵向**预算：槽高按终端行数分配（designing `layout` 模块
  「短终端仍须看见 busy Working」）；横向无对应机制。
- 断点判定目前隐式分散在各渲染函数内部，无共享常量或函数。

## 参照手法

成熟实现的通行做法：

1. 定义少量**具名断点档位**（如 <44 / <70 / <80 / <120 / ≥120），
   全组件引用同一档位判定函数，不在组件内裸写像素/列数比较；
2. 每个信息字段声明自己的**降级链**（完整 → 紧凑 → 隐藏），
   档位决定走到链条哪一步；
3. designing states 以真实列宽命名样例，评审时逐档目检。

## 引入设计（规格草案）

- DESIGN.md 增补「横向断点档位表」：五档（极窄/窄/标准/宽/极宽）×
  元素清单矩阵（保留/紧凑/隐藏三态）。
- 布局根提供单点判定函数 `width_tier(cols) -> Tier`；组件只允许消费
  Tier，禁止自写阈值比较。
- 首批对齐对象：footer、status、models picker、editor placeholder；
  transcript 工具行次期。
- 既有规则迁移以「行为不变优先」：先建表映射现状，再谈调整。

## 决策点

1. 五档边界列数（建议以现有组件实际行为反推，而非先验拍板）；
2. Tier 是否暴露给 designing states 命名（states 以档位名组织样例便于评审）；
3. 极窄档是否承诺全功能可达（建议否：极窄只保证核心对话流可用）。

## 影响面

- DESIGN.md 文档 + 布局根函数 + 3~5 个组件分支重排；
- designing 多模块 states 增补；harness 快照随动；无协议变更。
