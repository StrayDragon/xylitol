---
depends_on: []
---

# Client-Local 存储通道（storage channel）：多实例共享的面本地状态

## Why

产品 TUI 的 client-local 状态（editor ↑/↓ 历史、activity fold 设置等）目前各自散点持久化，
没有统一通道：跨实例一致性无保障、新增一类状态就要重造一遍「读-写-原子性」轮子。
多个 TUI 实例（以及未来第二产品面）共享偏好/布局状态是已出现的需求。

外部参照过的成熟做法：磁盘为真值 + 文件锁串行化写 + watch 防抖重载 +
通道分桶，零 IPC、零数据库即可支撑多实例一致，并让开发构建与正式构建
并行调试互不污染（外部对照见 research 笔记）。

## What Changes

- 新增面本地存储通道：JSON 文件为真值；写路径经文件锁串行化（读盘 → mutate → 原子写）；
  外部变更经文件 watch 防抖后整体重载。
- 存储路径按 **channel 分桶**（如 dev / 默认），不同构建通道的客户端状态互不污染；
  同 channel 多实例天然共享。
- 数组类条目按稳定 key reconcile，保对象身份（避免无谓的全量重建）。
- 既有散点 client-local 持久化点（editor 历史、fold 设置等）逐步迁入本通道，
  行为不变、落点收敛。

## 非目标

- 不承载会话数据 / host 侧状态（那是 Host 职责）。
- 不做网络同步或跨设备复制。
- 不引入通用配置系统（与运行时即时设置分界保持现状）。

## Impact

- 主仓 app 层新增小模块 + `packages/xylitol-tui` 可能暴露最小接口（渲染层不感知存储细节）。
- 迁移点逐个接线，每点有 harness 护栏；BDD 补「双实例写同一通道互相可见」场景。

## Further Notes

- 一手机制摘录与落点分析（外部实现对照）：[research/storage-channel-notes.md](./research/storage-channel-notes.md)
