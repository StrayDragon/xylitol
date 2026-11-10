# Design — 压缩边界与一次性通知

## 1. 目标与边界

- 边界推导单点；一次性通知发送成功才置位；压缩后不变量重建合一。
- 边界：不改切点算法本身；不动 compaction 配置。

## 2. 决策摘要

| 决策 | 内容 | 状态 |
|---|---|---|
| 边界单点 | boundary_start(entries) -> Boundary { None, At(usize) } 供 prepare/compact 共用；末条判定先跳过非消息 entry | 定稿方向 |
| 通知门 | NoticeGate 发送成功后置位；失败保留未置位 + 重试上限 | 定稿方向 |
| overflow 复位 | runtime 侧复位点随 c3240 run 边界复位一并落，本 change 只收敛 compaction 侧接口 | 推荐，待 review |
| 切点哨兵 | CutPointResult { cut: Option<usize>, split_turn: bool }；isize 哨兵删除 | 定稿方向 |
| 不变量重建 | ensure_state_after_compact 单次加载逐项补齐；失败写待重试标记 + 日志 | 定稿方向 |
| rider | 角色白名单合一；0 哨兵改 Option | 定稿方向 |

## 3. 通知门

NoticeGate { emitted, retries }：emit 闭包成功才置 emitted；失败 retries 递增，达上限置 emitted 并 warn（放弃而非永久重试）。两处调用方的 &mut bool 标志全部替换。

## 4. 测试边界（seam）

- 单测：两调用方边界一致性（同 entries 断言同 Boundary）；通知失败注入；重建后四类状态齐全。
- BDD：domain-compaction 场景回归。

## 5. 风险与回滚

- 通知时序改为成功才置位，重复通知窗口极小化；回滚三片独立。
