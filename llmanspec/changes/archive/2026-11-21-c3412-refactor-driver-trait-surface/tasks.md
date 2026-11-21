# Tasks：XyDriver 表面收窄

> 依赖 change：无（可与 c3410 并行）。needs_specs_change: false（纯实现层）。
> seam：公共函数 / 类型边界（runtime driver 模块）+ 全仓编译；无可观察行为变化，BDD 回归地板护绿。

- [x] T1: 二次普查（强制门）——重验剪贴板三件与 attach 六件在 runtime 侧无真实实现 / 消费者（rg 快照更新进 research/）；发现任何真实路径 → STOP 并转行为变更流程，不得静默删改。
- [x] T2: 删剪贴板三件（driver/proto.rs）——全仓编译绿；app-tui 镜像不动。[blocked-by: T1]
- [x] T3: 删 attach 六件（driver/proto.rs；XyInProcessDriver 若有 override 一并删）。[blocked-by: T1]
- [x] T4: session_store() 收窄——kernel bootstrap 直供 Arc<dyn XySessionStore>；react 内部 2 处改内部引用；app-tui 4 处改 attach / kernel 提供口；thoughtdag 跨仓适配点清单写入 research/。[blocked-by: T2, T3]
- [x] T5: XySessionOps trait 升格——session.rs 固有会话操作按域分列（树 / 条目 / 命名 / 生命周期），XyInProcessDriver 实现，kernel command_exec 改走 trait 对象。[blocked-by: T2, T3]
- [x] T6: 全量验证门——nextest（runtime / kernel / app-tui）+ just qa 全绿（回归地板）。[blocked-by: T4, T5]
