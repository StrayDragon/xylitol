# Design — c468-demo-steer-followup

## 取舍

| 选项 | 结论 |
|---|---|
| 忙碌二次 Enter 开新轮 | **否** — 会清脚本、丢工具/回复；改为 steer |
| steer 是否立刻跑 | **否** — 入队，当前轮结束后 apply（demo 近似「下一迭代」） |
| follow-up 是否挂树 | **否** — 仅系统提示；空闲 `commit_user_turn` 时再挂 user 节点 |
| 是否调 Driver | **否** — 仅 agent_demo 假队列；产品走 c461 seam |

## 与 c461 / ati3 对齐

- 键位与 `app-tui-input` ati3、`keybindings.md` 一致。
- demo 不实现真实 ReAct drain 时机；只保证「不打断 + 两队列 + 空闲/轮末 drain」。
