# Design — c645 产品 Shift+F → 新 session fork

## 语义选型（已拍板 A）

| | demo `ast5` | 产品 c645（A） | pi |
|---|---|---|---|
| 会话 | **同** session，改 leaf | **新** child session 文件 | `/fork` → `createBranchedSession` |
| API | 本地 `fork_from_history` | `Driver::fork_session` + `switch_session` | `runtimeHost.fork` |
| 持久化 | 无新文件 | 新 JSONL；**父文件只读** | 新 `.jsonl`；`parentSession`→原路径 |

**MUST NOT** 把产品 Shift+F 做成 demo 同会话。

---

## 存储合约（防写坏 · 对齐 pi）

### pi 真值（`session-manager.createBranchedSession` + `agent-session-runtime.fork`）

1. **只建新文件**，不改写父 session JSONL。
2. Header：`type=session` + 新 `id` + **`parentSession` = 父会话路径/标识**。
3. 内容 = **`getBranch(leafId)`：root→leaf 的 parentId 链**，**不是** JSONL 文件序 `entries[0..=i]`。
4. 写入前 **重链 `parentId`** 成线性路径（去掉路径外兄弟；pi 还会剥 label 再重挂）。
5. **默认 `/fork`（user 消息，`position: "before"`）**：
   - `targetLeafId = selectedUser.parentId`（根 user → 空会话 + 仅 header）
   - **不把**被选 user 行拷进 child
   - 预填 editor = 该 user 正文；下一次提交在 child 的 leaf 下长出新分支
6. `/clone` 才用 `position: "at"`（含选中节点整段路径）。

### xylitol 现状缺口（`SessionManager::fork_inner`）

| 现状 | 风险 |
|---|---|
| `parent_entries[..=fork_index]` **文件序**切片 | 已分叉会话会把**旁路兄弟**拷进 child，或漏掉路径上更早写入但树序不同的节点 → **存储坏 / 树坏** |
| 总是 **含** `at_entry_id` | 与 pi user-fork「before」不一致（会把选中 user 也拷走） |
| `branch_summary` 概括文件序「之后」条目 | 非 pi `createBranchedSession`；旁路摘要易误导 |
| 已有 `get_branch` | **应用层未用于 fork** |

### c645 MUST（实现时改 store，不只接线 TUI）

1. Fork 内容 **MUST** 来自 `get_branch(session, leaf)`（或等价），**MUST NOT** 仅按文件下标切片。
2. 写入 child 前 **MUST** 重链 `parent_id`（路径内线性；无孤儿、无旁路兄弟）。
3. **MUST NOT** mutate 父 session 文件 / 父 leaf。
4. Header **MUST** 设 `parent_session` 指向父 id（现有 `create(..., Some(parent_id))`）。
5. **产品树 Shift+F**：
   - 选中 **user** → `ForkPosition::Before`（pi `/fork`）：leaf=parent；预填正文；user 不进 child
   - 选中 **非 user** → `ForkPosition::At`（pi `/clone` 形态）：路径 **含** 选中节点；不预填 user 正文
6. `switch_session(child)` 后 leaf = child 路径末；重建 scrollback；父 UI 状态丢弃。

### 证据测（防回归写坏）

- 父会话已有 **兄弟分支**（同 parent 下两子）→ fork 于一侧路径 → child **不含**另一侧兄弟。
- 父 JSONL 字节/entry 数 fork 后不变。
- child header `parent_session == parent_id`；路径上 `parent_id` 链无环、无指向未拷贝 id。
- user-before：child 无该 user id；editor 有其正文。

> 既有 BDD「在记录 5 处分叉」偏文件序语义且 step 现为 stub——本 change **改合约**时同步改 feature/单测到 **branch 路径**，避免把坏语义测死。

---

## 接线（对齐 travel）

```text
Shift+F（树槽，选中 user）
  → pending_tree_fork = selected_id
  → effects::drain_pending
       leaf = parent(user) | None
       Driver.fork_session_via_branch(leaf) → child_id   // 内部 get_branch+重链
       Driver.switch_session(child_id)
       host: close tree · get_messages · rebuild scrollback
             · prefill user text · status「Forked to new session」
```

`fork_session` ** alone 不切换** active → 产品 MUST 显式 `switch_session`。

## Non-goals

- demo `ast5` 同会话 fork（保持）
- annotation；fork 选目录 UI
- 在父文件内「就地分叉」而不建新 session
