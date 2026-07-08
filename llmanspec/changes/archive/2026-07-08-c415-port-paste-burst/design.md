# Design — c415-port-paste-burst

> 单一设计决策：时间源注入方式。pi 用 `Date.now()` 硬编码，xy 用 `Instant` 参数注入。

## 决策：方法接受 `Instant` 参数，而非内部持有 Clock

### 选项

| 方案 | 做法 | 结论 |
|---|---|---|
| **A. Instant 参数注入** | `on_plain_char(now: Instant)`，调用方传 `clock.now()` | ✅ 采用 |
| B. 持有 `Box<dyn Clock>` | `PasteBurst::new(clock)`，内部调 `self.clock.now()` | ❌ 拒绝 |

### 理由

- **PasteBurst 是纯状态机**，无 I/O。给它塞 `Box<dyn Clock>` 会引入不必要的所有权/生命周期复杂度（Arc? Box? &'a?），而它只在 4 个方法里需要「当前时间」。
- **参数注入更可测**：测试直接构造 `Instant` 序列（`Instant::now()` 基准 + `+ Duration`），甚至不需要 MockClock——`Instant` 本身就是确定性值类型。这比 pi 的 `Date.now()`（mock.timers 冻结）更简单。
- **与 c405 Clock trait 互补**：editor（未来移植）持有 `Clock`，在调用 `paste_burst.on_plain_char(clock.now())` 时传值。PasteBurst 不需要知道 Clock 的存在。

### 后果

- PasteBurst 完全无状态依赖（除自身 4 个字段），可 `#[derive(Default)]`。
- 测试用 `Instant::now()` 基准 + `+ Duration::from_millis(N)` 构造时间序列，无需 MockClock（比 c405 tt04 的 MockClock 更轻——因为 PasteBurst 不持有 clock）。

## 非目标

- 不集成进 editor（c420 范围）
- 不处理 bracketed paste（stdin_buffer/editor 职责）
