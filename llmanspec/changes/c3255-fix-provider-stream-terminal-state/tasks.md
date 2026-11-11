# Tasks — Provider 流终态

> 实施者导读：design §1 缺陷矩阵含核证状态；§2 六项修复与任务一一对应。收口走 finalize，不列为任务。

- [ ] T1 spec 终态约束条款 + 三方言 fixture 矩阵骨架
- [ ] T2 chunk_stream_check 校验器（sdk 纯函数 + 全转移单测）
- [ ] T3 三适配器统一 StreamOutcome（终态内聚、终态后零产出、乱序显式错误）
- [ ] T4 Completions 修正（Done 移出 choices 循环、EOF 报错）+ 中断 flush（空参 End + 流错误）
- [ ] T5 rider：trace finalize 属性拦截（两 crate）；fake Cyclic 回绕含第 0 步
- [ ] T6 全量回归 [blocked-by: T1, T2, T3, T4, T5]
  SSE 矩阵 BDD；is_retryable 分类回归；just fmt / just lint / just test；llman-sdd validate --all --strict。
