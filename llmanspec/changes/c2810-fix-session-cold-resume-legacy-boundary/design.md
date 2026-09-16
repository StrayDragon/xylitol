# 设计：Session 冷段按需索引与旧格式边界

## 约束

- manifest 仍是 session 恢复的唯一提交指针；未被 manifest 引用的 JSONL、sidecar
  或临时文件不得影响 `load`、resume、list。
- 只在单进程单写者假设内提供一致性；不新增跨进程文件锁承诺。
- sidecar 是 sealed 段的实现细节，不改变逻辑 JSONL export/import 的输出形状。
- 旧 v7 manifest 没有 `indexPath` 时必须继续可读，按现有扫描路径回退。
- sidecar 缺失、JSON 损坏或无法用于候选筛选时必须回退扫描，不能因为索引 false
  negative 截断 leaf branch 或错误地产生 interrupted bash。
- 旧 v6/v5 `.jsonl` 不再自动迁移；不支持错误必须可操作，源文件保留，显式 delete
  才允许删除。

## Sidecar 形状

每个被新 manifest 引用的 sealed 段旁生成一个 immutable JSON sidecar：

```json
{
  "entryIds": ["..."],
  "doneBashIds": ["..."]
}
```

`SessionSegment` 增加可选的 camelCase `indexPath`。active 段不要求 sidecar；
active 每次普通 append 直接读取。manifest path validation 同时约束 segment path
与 index path 必须位于 session 根目录内，且不能重复引用。

sidecar 在 seal 时从同一批 prefix entries 构建：

- `entryIds` 收录该段所有非空 entry id；
- `doneBashIds` 复用协议层的 bash message 解析，只收录状态为 done 且 id 非空的
  bash；
- 输出排序/去重保持稳定，便于诊断和测试。

## Seal 提交顺序

1. 读取 active，构造 cold prefix、new active 和带 `indexPath` 的 cold descriptor。
2. 原子写入并同步 cold JSONL。
3. 原子写入并同步 sidecar。
4. 原子写入并同步 new active。
5. 原子切换 manifest。
6. 仅在 manifest 成功后删除旧 active；任何第 2–5 步失败都清理本轮 cold、
   sidecar、active orphan。

`replace_entries` 清理旧布局时也删除旧 segment descriptor 的 `indexPath`，避免
重写/修复流程留下永久 orphan sidecar。

## Leaf resolver

`load_branch_entries` 仍总是读取 active。沿 parentId 向上查找时：

1. 对有合法 sidecar 的 sealed 段先用 `entryIds` 筛候选；
2. 只读取命中的候选段，并缓存本次调用已读段；
3. 没有 sidecar 的段、sidecar 读取/解析失败、或候选未找到目标时回退读取未读
   sealed 段，保证索引损坏不会造成 false negative；
4. 保留既有 compaction cut、leaf anchor、branch sibling 排除语义。

完整 `load_entries`、export、inspect 继续按逻辑顺序读取全部被 manifest 引用的段。

## Bash done 配对

在 `XySessionStore` 增加带默认实现的 `load_done_bash_ids` 端口。默认实现从
`load_entries` 计算，保证 fake store 和其它实现无需同步修改；`SessionManager`
实现读取 manifest：

- active 段直接扫描；
- sealed 段优先合并 sidecar `doneBashIds`；
- 旧 manifest、缺失/损坏 sidecar 回退读取对应 sealed JSONL。

ReAct history seeding 与 `build_session_context` 改用这个端口，再对 leaf branch
执行已有 `fold_interrupted_bash_rows`。因此 done 行位于 cold、running 行位于
active 时仍保持 session-wide 配对，同时不再为了 done 集合全量解析 cold。

## Legacy boundary

当 session 目录没有 v7 manifest、但存在旧 `{id}.jsonl` 时，所有自动恢复和写入入口
统一返回“旧 session storage 不受支持，请使用旧版本导出后再导入”的可操作错误：

- 不调用 migration、headerless repair 或 cleanup retry；
- 不创建一个同 id 的新 v7 session 覆盖旧文件；
- 不在后台删除旧文件；`delete_session` 这类用户显式删除操作仍可清理它；
- list 可跳过该条目并记录有限诊断，不得把它当作已可 resume 的 v7 session。

保留协议层的 v6 JSONL 解析/导入能力时，必须明确它只服务于显式 import，不构成
SessionManager 的长期双读或 lazy migration 路径。

## 失败恢复与兼容矩阵

| 状态 | 读取行为 |
|---|---|
| 新 manifest + sidecar 完整 | 索引筛段，active + 命中 cold |
| 旧 manifest 无 `indexPath` | 既有扫描回退，逻辑行为不变 |
| sidecar 缺失/损坏 | warning + 对应段扫描 |
| manifest 提交前留下 orphan | 忽略，不进入逻辑流 |
| 旧 v6/v5/未知 legacy 文件 | 拒绝自动恢复，返回可操作错误并保留文件 |
| 显式 import 旧 JSONL | 由 import parser 单独处理，不触发 storage migration |

## 测试 seam

测试复用现有 `SessionManager` / `XySessionStore` 与
`tests/features` 的 agent-session-store seam，不新建平行存储 runner。纯 infra
单测负责 sidecar 序列化、manifest path、失败清理与 resolver fallback；BDD/集成
场景负责 resume/context、legacy boundary 和逻辑可观察结果。
