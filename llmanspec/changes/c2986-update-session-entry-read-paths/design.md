# Design

## Evidence

- `crates/xylitol-app-tui/src/effects/mod.rs:39-42`：`session_entries` → `session_entries_page(driver, None, None)`，即 `limit=None` 全量。
- 调用点：
  - `effects/pending_ui/tree.rs:66` / `:106` / `:163`：session tree 构建与 travel；
  - `effects/slash/compact.rs:36`：压缩前读取；
  - `effects/mod.rs:375`：其他 UI 路径。
- `crates/xylitol-app-tui/src/driver/types.rs:152`：`estimate_from_session_entries` 也吃全量 entries。
- `llmanspec/specs/agent-session-store/agent-session-store.feature`：已存在分页能力（`load_entries_page`）。

## Per-path analysis

| 路径 | 是否必须全量 | 可行收敛 |
|---|---|---|
| session tree | 否，树只需父子关系 | 用 `SessionTree` 查询或按页构建 |
| fork | 否，只需目标条目到叶子的分支 | 定向分支查询（若 store 支持） |
| compact | 是（摘要需要完整上下文） | 保留全量，注明原因 |
| estimate | 否，可基于已有窗口/分页 | 分页或增量估计 |
| slash 其他 | 视命令而定 | 按需 |

## Options

### Option A：逐路径收敛（推荐）

- tree 改用 `SessionTree` unary（已有命令）或分页；
- fork 增加“分支查询”或按页向前读取直到目标条目；
- compact 保留全量；
- estimate 用分页/增量。

优点：IO/内存下降，行为不变。
缺点：需要逐个路径验证行为一致性；fork 可能需要 store 新 API。

### Option B：只改 tree，其余保留

- 优点：改动小。
- 缺点：fork/estimate 仍全量。

### Option C：接受现状

- 优点：零风险。
- 缺点：长会话 IO 峰值持续。

## Recommendation

采用 **Option A**，但按收益排序：

1. session tree（收益最大、语义最清晰）；
2. fork（需要分支查询，收益中等）；
3. estimate（可用分页增量）；
4. compact 保留全量并注明。

## Test seam

- 单测/基准：fake driver 记录 `GetMessages` 的 `limit` 参数，断言 tree/fork 不再传 `None`。
- 行为回归：tree 结构、fork 结果、compact 摘要与改动前一致。
- 长会话基准可作为一次性 `lab_` 脚本。

## Risks

- tree 的 `SessionTree` 查询可能不包含 travel 所需的全部字段；需要核对。
- fork 的分支查询若不存在，新增 store API 会影响 SDK。
- compact 全量读取是语义要求，不能为了省 IO 改成增量（会改变摘要质量）。

## Open questions

1. fork 是否需要新增 store 分支查询 API？
2. estimate 是否允许用分页近似（可能影响 token 估计精度）？
3. tree 是否直接复用 `SessionTree` unary，还是保留端侧构建？
