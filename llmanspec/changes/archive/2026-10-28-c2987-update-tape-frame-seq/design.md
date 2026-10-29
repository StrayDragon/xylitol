# Design

## Evidence

- `crates/xylitol-serve/src/occupancy/host.rs:688`：tape 帧 `seq: self.journal.lock().await.max_seq()`。
- `llmanspec/specs/server-core/server-core.feature:107`：流式 tape 不得写入 journal、不得消耗 seq；journal 只承载事实事件。
- `c2954`：tape 不写 journal、不耗 seq；中段重连用 `partial` 快照恢复当前回合。
- TUI 当前不做 seq 去重，所以同 seq 无可见问题。

## Semantics today

- `seq` 表示“当前事实事件位置”；
- 同一批 tape 帧共享同一个 `seq`；
- 客户端若用 `seq <= last_seq` 过滤（常见去重手法），会误丢实时 tape。

## Options

### Option A：独立 tape 序号（推荐，若要做长期协议）

- tape 帧增加 `frame_seq`（或 `tape_seq`）字段，单调递增、不写入 journal；
- 客户端用 `frame_seq` 去重/排序 tape，用 `seq` 对齐事实事件；
- 优点：语义清晰，未来客户端可安全去重。
- 缺点：协议形状变更，需要 `PROTOCOL_VERSION` bump；客户端要适配。

### Option B：保留 journal seq，明确“不得按 seq 去重 tape”

- 在 `server-core` / 协议文档写明 tape 帧的 `seq` 只是事实位置快照，不唯一；
- 客户端必须按帧内容/到达顺序处理 tape，不得用 `seq` 过滤；
- 优点：零协议变更；与现有客户端行为一致。
- 缺点：依赖约定而非结构保证；未来客户端容易踩坑。

### Option C：tape 消耗 seq

- 每帧分配唯一 seq；
- 优点：唯一性直接。
- 缺点：与 spec“tape 不得消耗 seq”冲突；journal 重放语义被污染。

## Recommendation

- 若近期没有“客户端按 seq 去重 tape”的需求：选 **Option B**，补文档与测试；
- 若计划长期支持多客户端/重连去重：选 **Option A**，并接受协议 bump。

当前建议 **B**（成本最低、与现状一致），把 A 留到确有客户端需求时再做。

## Test seam

- 单测：同一批 tape 帧的 `seq` 相同（现状）且事实事件 seq 单调；
- 若选 A：tape `frame_seq` 单调递增、不推进 journal seq；
- 客户端去重测试：按文档约定处理 tape，不因同 seq 丢帧。

## Risks

- Option A 的协议 bump 会影响旧客户端；需要版本协商与兼容窗口。
- Option B 把正确性寄托在文档约定上，未来容易回归。
- tape 帧与事实事件的相对顺序必须保持：无论选哪个，帧到达顺序不能乱。

## Open questions

1. 是否已有客户端/未来计划需要按 seq 去重 tape？
2. 若选 A，字段名用 `frame_seq` 还是 `tape_seq`？是否需要 bump `PROTOCOL_VERSION`？
3. 是否需要在 `host.describe` 里宣告 tape seq 语义？

## Decision (implemented)

- 采用 Option B：不新增 `frame_seq`，在 `server-core` 的 `r1798` 明确 tape 帧 `seq` 是事实位置快照、不唯一，客户端不得按 seq 去重/过滤。
- serve 测试断言同批 tape 共享当前事实 seq 且不推进 journal。
- `PROTOCOL_VERSION` 不变。
