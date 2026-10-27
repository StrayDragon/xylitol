# Design: c2946

## 1. 为什么是注册表派生，不是 rutis isolate

`Ctx::isolate(key, label)` 是框架的**服务**作用域：同类型服务按 label 并存。它对「按会话换 compaction / permission」是正确的，但对「这条中间件链属于哪个会话」不对位：

- 链是**数据**（一组有序 `Arc<dyn LoopMiddleware>`），不是**服务**；
- `isolate` 返回的 `Ctx` 复用原 fiber（`ctx.rs` 传 `self.0.fiber.clone()`），绑定活到根 ctx 卸载；
- Host 的 `sessions: HashMap` 只增不删，`SessionSlot` 无 `Drop`，`Disposer` 无 `Drop` 自动清理 —— 框架级路径下「随会话释放」不可达。

数据派生的释放语义天然成立：槽 `Arc` 掉落即释放，无需 dispose 点。

## 2. `derive()` 语义

```text
parent: [a(order 10), b(order 20)]
child = parent.derive()
  → child 冻结为 [a, b]，此后 parent 增删不影响 child
child.register(c)  → child = [a, b, c]，parent 仍 [a, b]
```

- **冻结而非引用**：子持有的是父链**当前快照**的 `Arc` 克隆，不是父的活引用。这与「run 启动快照」同一哲学（c2944 已定：链在 run 启动时快照）。
- **本地可注册**：子可继续 `register` / `unregister`，只影响自己。
- **不合并后续**：父后来注册的中间件**不会**流入已存在的子。理由：Host 槽创建时机与插件挂载时机不同，若合并会让「已存在 session 突然多出一条中间件」，违背「已存在 session 不受新挂中间件影响」。

实现上 `derive()` 复用 `snapshot()` 的有序结果，按 `order` + `seq` 重建 entry 列表；`seq` 从父的最大值 +1 续，保证子内新增项排在父项之后同 order 时不越序。

## 3. Host 接线

`SessionSlot` 增加 `loop_registry: Option<Arc<LoopMiddlewareRegistry>>`，在 `SessionSlot::new` 时从 `HostState` 的 root registry 派生。`materialize_writer_at` 把槽内那份交给 driver。

**顺序问题**：`SessionSlot::new` 目前只接 `session_id` + downlink，拿不到 root registry。两条路：

- **A**：`SessionSlot::new` 多接一个 `Option<Arc<LoopMiddlewareRegistry>>` 参数，由 `HostState::slot()` 传入。
- **B**：槽不持有 registry，`materialize_writer_at` 时现派生。

取 **A**：槽创建即冻结快照，语义与「槽创建时派生」一致；B 会让「槽已存在但 driver 后建」时拿到更新的父链，破坏冻结语义。

## 4. 观测口径

`loop.middleware.scope` 属性取值 `root` / `session`（不做全 id，避免 trace 泄漏会话标识）。由 `AgentRuntime` 记录其链来源：`set_loop_middleware` 时带上来源标记。

最小实现：`AgentRuntime` 增 `loop_scope: LoopScope`（`Root` / `Session`），`set_loop_middleware` 保持签名不变（默认 `Root`），新增 `set_loop_middleware_scoped(registry, scope)` 供 Host 用。middleware span 的 properties 增加该值。

## 5. embed 入口

```rust
pub fn with_loop_registry(
    driver: XyInProcessDriver,
    registry: Arc<LoopMiddlewareRegistry>,
) -> XyInProcessDriver
```

只做 `driver.set_loop_middleware(Some(registry))` 并返回，不引入 rutis 类型。嵌入者自建 `LoopMiddlewareRegistry::new()` 注册后传入。

## 6. 测试

| 层 | 测试 |
|---|---|
| sdk 单测 | `derive` 冻结父快照、子本地注册不影响父、父后续注册不流入子 |
| kernel 契约 | 两个 `SessionSlot` 各自 registry 互不影响；root 为空时 driver 链为空 |
| BDD | 两会话各挂不同中间件，事件流可区分 |

## 7. 不做什么

不引入 `Disposer` / 显式释放点：数据派生不需要。不把 `isolate` 塞进来做「两套作用域并存」——那是未来服务级 change 的事。
