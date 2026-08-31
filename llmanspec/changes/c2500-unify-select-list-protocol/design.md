# Design — c2500 SelectList 结构下沉（行为零变化）

## 硬约束

- **行为零变化**：所有槽的键位、渲染、过滤语义重构前后完全一致；harness 快照零 diff 是验收线。
- resume 自绘 panel（`session_resume/panel.rs` + `search.rs`）不在改动范围。
- 包层公开 API 不加新字段（group / details / action 不做）。

## 现状核对（代码事实）

- 过滤语义已分叉：`SelectList::set_filter` = `value` 前缀匹配（lowercase）；
  models 槽走包层 `fuzzy_filter`。**两者都保留**，抽取时参数化。
- 五槽重复面：`SelectList::new(items, max_visible, theme, layout)` 组装、
  `SelectListTheme` 取用、`SelectListLayoutOptions` / `TruncatePrimaryContext`
  构造、description 截断与滚动提示渲染。

## 决策

- **D1 下沉边界**：只抽「各槽逐字重复或仅参数不同」的构造与渲染辅助；
  行为有差异的地方以参数表达（如过滤函数作为参数），**不做语义归一**。
- **D2 过滤纯函数**：`SelectList::set_filter` 的前缀匹配抽为包层纯函数（可直测）；
  `fuzzy_filter` 保持既有导出不动；两函数并存，调用点各自不变。
- **D3 主题与布局**：`SelectListTheme` / 布局参数的默认值收敛为共享构造路径，
  各槽显式覆盖项保持显式。
- **D4 验收口径**：重构前后——①五槽 harness 快照零 diff；②包层既有单测零改动通过
  （允许新增纯函数单测）；③`cargo test` 全绿。

## 非目标

resume panel、分组头、details、action 条、命令化、预览侧栏（沿 proposal）。
