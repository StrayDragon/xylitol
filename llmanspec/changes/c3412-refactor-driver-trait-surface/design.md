# Design：XyDriver 表面收窄

真源：c3410 design.md D2；本文只写本 change 特有形状。

## 普查快照（2026-11-20，实施 T1 须重验）

- 剪贴板三件：runtime trait 仅默认 unsupported 实现；真实实现在 app-tui/src/driver/clipboard.rs 与 clipboard/ 模块；调用点 effects/pending_ui/clipboard.rs:32、effects/slash/misc.rs:101 均走镜像 driver。
- attach 六件：消费者全在 app-tui 镜像（lib.rs:142,377,391,546,553；remote.rs 各实现）。
- session_store()：runtime react 内部 2 处（react/mod.rs:143,593）、kernel bootstrap.rs:950、app-tui 4 处。

## 目标形状

1. trait 删 9 方法（剪贴板 3 + attach 6）；app-tui 镜像零改动。
2. session_store() 从 trait 删除；store 由 kernel 直供：bootstrap 装配处向 embed / app 客户端传 Arc<dyn XySessionStore>；react 内部改持内部引用；app-tui 改从 attach 面 / kernel 提供口获取。
3. XySessionOps trait 升格：session.rs 固有会话操作按域分列（树 / 条目 / 命名 / 生命周期），XyInProcessDriver 实现；kernel command_exec 与 thoughtdag 固有调用点改走 trait 对象。

## 不变式

app-tui 镜像 trait 与 wire 路径零改动；可观察行为零变化（回归地板护绿）；thoughtdag lockstep（固有调用点改 trait 后为其编译期适配点）。

## specs / BDD 绑定方案

- needs_specs_change: false——无 live specs/** 编辑（纯实现层）。
- 逃生条款：T1 普查若发现真实路径 → STOP，转 needs_specs_change: true 并在本节补绑定方案。
- BDD：回归地板护绿，无新场景、无绑定变更。
