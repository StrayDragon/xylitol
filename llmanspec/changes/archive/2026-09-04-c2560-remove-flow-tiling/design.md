# Design: c2560-remove-flow-tiling

## 决策

- **整体移除**时态图（视图 + flow.yaml 契约 + React 依赖），不做「保留数据去视图」的中间态：flow.yaml 无渲染面即死数据，违背 pre-0.0.1 无死数据纪律。
- **交互原型呈现回归固定态 chips**：与产品模块同构（adp9 语境），键盘黑盒禁令由现状（黑盒已删）自然满足，不另立条款。
- **spec 只删不增**：adp10 整条移除；adp9（static-slots）不涉及 flow；其余 adp 条款不动。
- shell 页 label 细分与真实会话导出通道随本 change 实现但**不落 spec**（呈现层细节，非 MUST 语义）。

## 移除清单

- `designing/app/src/flow.ts` / `flow-canvas.tsx`；main.ts 内 flow 机制与 `__flow__` 路由
- 依赖：react / react-dom / @xyflow/react / @types/react*；tsconfig `jsx` 回退
- `designing/tui-lab/modules/*/flow.yaml`
- AGENTS / AGENT-INDEX 中时态图与 flow 标注

## 保留

- `states/*.yaml`（lab 固定态 chips 继续渲染）；shell 页与区域注解；帧导出器（新增真实会话通道：环境变量指定磁盘会话，默认合成种子保持可复现）。
