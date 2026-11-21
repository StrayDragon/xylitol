# Design：SamplerActor 与并行模型调用

真源：c3410 design.md D6 / §4.4；本文只写本 change 特有形状。

## SamplerActor 协议

Submit{request, respond_to} / Cancel{request_id} / UpdateConfig{partial} / IsActive / ActiveCount——每会话槽一个；select biased（join_next 先清理再收新）。

## 并行语义

- 同 turn 多请求：turn 任务发 Submit，各自 child token；结果汇聚顺序 = 发起序（与 r1046 源序原则同构），完成事件按完成序交错；汇聚细节【细化】。
- 重试梯：r3021 分类 + retry.rs 迁入子任务；AutoRetryStart / End 事件语义不变（r3304）。
- 429 三层：层 1 子任务内重试梯；层 2 会话预算缓做（子代理只读很轻）；层 3 进程级 Semaphore 注入点预留（默认不限）。

## 旁路登记

generate_session_name（react/mod.rs:101 先例）与 compaction 摘要改走 Submit（保持 StageNameConfig 超时 / 预算语义）——统一记账首个收益。

## 热重载

同 turn 全部请求（含并行）用 turn 起始绑定（r1042 扩展）；UpdateConfig 仅对新 Submit 生效；可 / 不可热换清单落 specs（c3410 §4.4【细化】）。

## seam

runtime 单测（Fake XyModel 并发时序断言，HoldModel 先例）+ react 既有场景回归。

## specs / BDD 绑定方案

- 采样记账规则场景（c3410 §5.1 新增，id 预留 r3370）step 定义落地：HoldModel 双请求并发时序断言 + AutoRetryStart / End 事件回归（既有 bindings 更新）。
- barrier_parallel 既有场景回归断言（并行采样不影响工具批语义）。
