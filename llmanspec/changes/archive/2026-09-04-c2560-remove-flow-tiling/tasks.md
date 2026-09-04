# Tasks: c2560-remove-flow-tiling

测试 seam：复用既有——designing 闸（check_tui_designing 进 just qa）、playground tsc/build、cargo 测试（lab_design_frame 探针）。无新 seam。

- [x] t1 specs：移除 adp10（rules_edit_acked）；`llman sdd validate` 结构绿
- [x] t2 移除时态图实现：flow.ts / flow-canvas.tsx / 各 flow.yaml / React 系依赖 / `__flow__` 路由与 live 机制残留；lab 回归固定态 chips
- [x] t3 shell 页收尾：header tabs 移除；区域 label 细分（transcript 按条目形态）；实验原型条目与区域条目同构；导出器真实会话通道（环境变量）
- [x] t4 文档：designing/AGENTS（去时态图、真实会话导出说明）；AGENT-INDEX 重生成
- [x] t5 门禁：tsc + just qa + 浏览器验收
