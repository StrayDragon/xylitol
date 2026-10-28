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

## Audit outcome (implemented)

| 路径 | 读取需求 | 决定 |
|---|---|---|
| session tree 打开 | 已有 `SessionTree` unary，不调用 `session_entries` | 无需改动 |
| footer token 可用性 | 只需判断“是否有条目” | 改为 `session_entries_page(Some(1))` |
| CLI 恢复回退（无本地 store） | 只需首屏窗口 | 改为 `session_entries_page(Some(HISTORY_PAGE))`，并透传 `has_more` |
| tree travel | 需按 `travel.leaf_id` 重建祖先路径；路径可能跨任意多页 | 保留全量（行为正确性优先） |
| tree fork | 需按 `entry_id` 取目标条目的角色与文本以决定 fork 位置与编辑器预填 | 保留全量（收敛需协议扩展） |
| fork 后切换会话 | 需按新会话叶分支重建滚动区 | 保留全量 |
| compact | 摘要需要完整上下文 | 保留全量 |
| estimate | token 估计基于完整上下文 | 保留全量 |

> travel / fork 若要收敛，需要 host 在 travel/fork 响应里直接返回所需条目或新增“按 entry_id 取条目”的 store 查询；这属于协议/端口扩展，不在本 change。

## Recommendation

采用逐路径收敛，按收益排序：先收敛 footer 可用性与 CLI 恢复回退；tree 打开已走 `SessionTree`；travel / fork / compact / estimate 保留全量并在本表记录理由。

## Test seam

- TUI harness：footer token 刷新仍能区分空/非空会话；CLI 恢复回退只取一页并保留 `has_more`。
- 行为回归：tree travel、fork、compact 摘要与改动前一致。
- 长会话基准可作为一次性 `lab_` 脚本。

## Risks

- tree 的 `SessionTree` 查询可能不包含 travel 所需的全部字段；需要核对。
- fork 的分支查询若不存在，新增 store API 会影响 SDK。
- compact 全量读取是语义要求，不能为了省 IO 改成增量（会改变摘要质量）。

## Open questions

1. fork 是否需要新增 store 分支查询 API？
2. estimate 是否允许用分页近似（可能影响 token 估计精度）？
3. tree 是否直接复用 `SessionTree` unary，还是保留端侧构建？
