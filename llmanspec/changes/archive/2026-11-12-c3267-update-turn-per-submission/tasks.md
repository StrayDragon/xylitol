# Tasks — 提交即 turn

> 实施者导读：design §0 词汇终态、§2 走查样例、§3 更名清单是验收基准。依赖 c3266 合入。收口走 finalize，不列为任务。

- [x] T1 spec 先行
  agent-runtime 新条款（提交即 turn / FIFO 逐条 / abort 语义重述）+ abort-queue-semantics 重写 + 概念层 run 残留清扫（五处+）；agent-hooks 事件名；agent-prompt 注入点跟名；agent-goal / agent-automation 措辞；app-tui busy 连续性；AGENTS 词汇段改写。验证：llman-sdd validate --specs --strict。
- [x] T2 循环改造
  settle → AgentEnd → FIFO 弹一条 → AgentStart 新 turn；follow-up drain 改 pop_front；steer 双路径（在途 decide_continue 注入 / settle 后入 FIFO）。单测：3 条 follow-up → 3 对事件、顺序断言；steer 两分支；abort 后弹出下一条。
- [x] T3 hook 事件更名 run_* → turn_*（一步到位不留别名）
  script_hook_ctx、hooks 插件、消费端、BDD。验证：旧串 rg 归零。
- [x] T4 c3248 注入点跟名
  run_start/run_end → turn_start/turn_end（枚举 / 配置校验 / 编译表 / spec 措辞）；turn_pre 谓词注释更新（每 turn 恰一次真）。
- [x] T5 TUI busy 连续性
  AgentEnd 时队列深度非零保持 busy（bridge/host 联动）；print 粒度回归。单测 + BDD。
- [x] T6 automation / goal 跟进
  tick 投递与 goal 续跑改述逐 turn；机制回归。
- [x] T7 全量回归 [blocked-by: T1,T2,T3,T4,T5,T6]
  just fmt / lint / test / qa；validate --all --strict；概念层 run 残留终检（标识符白名单外归零）。
