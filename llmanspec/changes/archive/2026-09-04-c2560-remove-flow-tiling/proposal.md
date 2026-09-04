---
rules_edit_acked: true
depends_on: []
branch: sdd/c2560-remove-flow-tiling
base_sha: 267e8fbfd94be05210cd17d7eec74c4e3cc8da78
checkpointed: true
checkpoint_sha: 267e8fbfd94be05210cd17d7eec74c4e3cc8da78
---

# 移除时态图（flow tiling）呈现

## Why

c2550 为 tui-lab 交互原型引入了声明式时态图（flow.yaml + xyflow 画布）。落地后实际使用发现：候选形态用固定态 chips 对照已足够，时态图引入 React 运行时与第二套渲染范式，维护成本高于收益（设计稿 playground 定位是人类查看与 prototype 验证，交互手感由 shell 页演示）。整体移除时态图呈现与 flow 数据契约，交互原型回归与产品模块同构的固定态呈现。

## What Changes

- 移除 `app-tui-design-playground` 的 `@req:adp10`（temporal-flow-tiling）：交互原型不再 MUST 提供声明式时态图；键盘黑盒禁令保留在 adp9 语境（现状已无黑盒模拟）
- 删除 designing/app 内时态图实现（flow/flow-canvas 模块、React 依赖）、各 tui-lab 模块 flow.yaml、`/tui-lab/<id>/__flow__` 路由
- tui-lab 候选呈现回归固定态 chips；playground 依赖回归 marked + yaml
- shell 页区域注解支持 label 细分（主 transcript 按条目形态拆分：用户消息 / 助手回复 / 折叠簇 / 滚动提示），帧导出器支持真实会话导出通道（环境变量指定磁盘会话），并同步文档

## Capabilities

- `app-tui-design-playground`：移除 adp10；其余 adp 条款不变

## Impact

- flow.yaml 数据契约废止；AGENT-INDEX 的 flow 标注随之消失
- designing/app 回归零框架（移除 React / @xyflow/react 依赖）
- 无产品代码改动；帧导出器（lab_design_frame）保留并增加真实会话导出通道
