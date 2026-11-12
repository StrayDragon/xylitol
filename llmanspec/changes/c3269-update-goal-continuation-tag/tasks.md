# Tasks — goal 续跑来源标记

> 实施者导读：design §0 是业界依据，§1 消息形态是验收基准。依赖 c3268 合入。收口走 finalize，不列为任务。

- [ ] T1 spec：agent-goal r3301 补措辞 + app-tui / cli-print 渲染条款
- [ ] T2 消息构造包标签 + 单测更新（goal_continue… 含标签断言；真人输入无标签不变量）
- [ ] T3 端侧渲染：与 c3248 共用标签检测；TUI 系统消息样式 + print 前缀样式；BDD / 快照
- [ ] T4 回归：just fmt / lint / test；validate c3269 --strict 绿
