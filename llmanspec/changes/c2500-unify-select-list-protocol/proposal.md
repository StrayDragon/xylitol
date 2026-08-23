---
depends_on: []
---

# SelectList 协议收敛：一个列表契约服务全部选择槽

## Why

包层已有 `SelectList/SelectItem` 抽象且被五个槽使用（import / models /
themes / mcp / resume），但能力停在「过滤 + 选中」：分组、action 条、
多行说明各自缺失或自绘。resume 面板因能力缺口完全绕开组件自绘
（搜索/分批/rename 全套私有实现）。每加一个列表类功能都在重造轮子，
行为细节（过滤语义、选中稳定性）开始漂移。

## What Changes

- 包层 `SelectItem` 增加可选 group 键与 `details` 多行说明；
  `SelectList` 支持分组头渲染与「组头不可选、过滤保组」规则。
- 新增 action 条协议（槽声明 chord+label，Tab 移焦点，回调归槽）。
- 过滤抽纯函数进包测试层；resume 自绘 panel 迁移到协议
  （分批加载 / rename / delete 变为 action 条动作，既有交互决议不变）。

## 非目标

- 不改任何槽的既有产品决议（PI 对齐项保持）；
- 不做预览侧栏（列为后续可选扩展）。

## Impact

- 包组件 + 单测；五槽接线改造；designing 对应模块 states 更新；
- 是 c2505 命令面板的前置（面板直接复用该协议）。

## Further Notes

- 现状核对（含 API 清单与使用分布表）：[research/select-list-protocol-notes.md](./research/select-list-protocol-notes.md)
