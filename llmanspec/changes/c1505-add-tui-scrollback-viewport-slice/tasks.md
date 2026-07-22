# Tasks: c1505-add-tui-scrollback-viewport-slice

> 实现闸：先跑 `post-c1509` B（大种子）确认 flatten 仍热，再勾选 T2+。

- [ ] T0: `post-c1509` profile B（±C）— 记录 `render_scrollback` / extend 是否仍随历史涨；结论写回 proposal
- [ ] T1: 定义 `ScrollbackViewport { start_line, height, overscan }`；`render_scrollback` 可选参数（默认全量）
- [ ] T2: follow-bottom 路径：按 cache 行数跳过屏外 entry；部分切入首个可见 entry
- [ ] T3: streaming tails / 贴底始终渲染；fold 变更失效窗口
- [ ] T4: host/layout 传入 upper 高度与 offset；默认行为与今日一致
- [ ] T5: harness — 长 scrollback 输出行数上界 + 贴底可见内容
- [ ] T6: `just test` / 相关 tui 闸；可选再跑 profile B 对比
- [ ] T7: proposal 状态 → applied；归档
