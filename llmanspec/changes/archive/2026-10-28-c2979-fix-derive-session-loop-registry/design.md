# Design

## Evidence

- `llmanspec/specs/agent-runtime/agent-runtime.feature:518`（`r2931`）：会话槽须持有自 root 派生的独立链，派生须冻结 root 当时的链快照。
- `crates/xylitol-serve/src/occupancy/host.rs:150-158`：`SessionSlot::with_seq(..., self.factory.loop_registry(), watermark)`。
- `crates/xylitol-kernel/src/occupancy/factory.rs:141-147`：`loop_registry()` 直接返回 `ctx.get::<LoopMiddlewareRegistry>()` 的 root `Arc`。
- `crates/xylitol-serve/src/occupancy/host.rs:908`：以 `LoopScope::Session` 挂上这个 root registry。
- `crates/xylitol-runtime/src/agent/capabilities/mod.rs:187-196`：`loop_chain()` 每次 run 调 `registry.snapshot()`，不是派生快照。
- `crates/xylitol-plugin-sdk/src/loop_middleware.rs:301-305`：`snapshot_scoped()` 只是给 `snapshot()` 结果打 scope 标记。
- `derive()` 仅出现在 `crates/xylitol-kernel/src/contract.rs:423-424` 与 SDK 单测。

## Problem shape

“Session 作用域”目前只是给 root 链打了一个 `scope=session` 标签，链内容仍来自共享 root。因此：

- root 在槽创建后注册的中间件会流入已存在槽（违反“冻结父快照”）；
- 槽内若未来允许注册，会直接影响 root 与其他槽（违反“互不影响”）。

因为目前没有 per-session 注册入口，这个问题没有用户可见症状，但它是 spec 承诺与实现之间的实质缺口。

## Options

### Option A：槽创建时派生（推荐）

```rust
SessionSlot::with_seq(
    session_id,
    downlink,
    factory.loop_registry().map(|r| r.derive()),
    watermark,
)
```

- 派生发生在槽创建时，满足“冻结 root 当时的链快照”。
- 后续 `set_loop_middleware_scoped(Some(derived), LoopScope::Session)`。
- reader driver 仍走 `factory.new_driver()` → root，不变。

优点：语义与 spec 一致；改动一行 + 测试；派生成本低（浅拷贝条目 + `Arc` 共享）。
缺点：无（当前没有 per-session 注册入口，行为不变）。

### Option B：首次 run 时惰性派生

- 槽先存 root，首次 run 前再 `derive()`。
- 优点：延迟成本。
- 缺点：槽创建到首次 run 之间的 root 注册仍会流入，不满足“槽创建时冻结”。不推荐。

### Option C：保留共享 root，修改 spec 放宽保证

- 把 `r2931` 改成“会话作用域仅标记，不保证隔离”。
- 优点：零代码。
- 缺点：放弃已承诺的隔离语义，未来 per-session 注册会踩坑。不推荐。

## Recommendation

采用 **Option A**。同时补一个 Host 层测试：

1. root 注册 `mw-a`；
2. 创建 slot-1、slot-2（各自派生）；
3. root 再注册 `mw-b`；
4. 断言 slot-1 / slot-2 的链都只含 `mw-a`；
5. 在 slot-1 的派生 registry 上注册 `mw-c`，断言 slot-2 与 root 都看不到 `mw-c`。

## Test seam

- `crates/xylitol-serve/src/occupancy/host.rs` 的 `#[cfg(test)]` 模块，或 `crates/xylitol-kernel/src/contract.rs` 已有的 registry 隔离测试扩展。
- 不需要 BDD：这是组件边界行为，单测更合适。

## Risks

- `SessionSlot` 在驱逐后会重建；每次重建都会重新派生，需确认不会把旧槽的 per-session 注册“复活”。当前没有 per-session 注册，风险为零；未来引入时要定义槽重建语义。
- `derive()` 保留 `next_seq`，派生 registry 后续注册不会与 root 冲突；需在测试里覆盖 order/seq 排序。
- 若其他代码路径直接读取 `slot.loop_registry` 并期望它是 root，需要一并核对（当前只有 `materialize_writer_at` 使用）。

## Open questions

1. 是否在本 change 一并暴露 per-session 注册 API？建议不做，保持最小修复。
2. reader driver 是否需要 Session 作用域？当前只读命令不需要链，保持 Root。

## Decision (implemented)

- 采用 Option A：`SessionSlot::with_seq` 在槽创建时 `Arc::new(root.derive())`。
- Host 层两槽隔离测试覆盖：root 后续注册不流入、槽内注册不回写。
