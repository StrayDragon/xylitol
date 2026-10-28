# Design

## Evidence

- `crates/xylitol-plugin-sdk/src/session/workflow.rs:9`：`const LEGACY_CUSTOM_TYPE_PLAN: &str = "plan";`
- `crates/xylitol-plugin-sdk/src/session/workflow.rs:131-133`：`latest_workflow` 同时匹配 `"workflow"` 与 `"plan"`。
- `crates/xylitol-plugin-sdk/src/session/checkpoint.rs:8`：`LEGACY_CUSTOM_TYPE_PLAN_CHECKPOINT`。
- `llmanspec/specs/agent-session-store/agent-session-store.feature:359` / `:365`：读取须兼容旧名的一次性迁移路径，新写入不得使用旧名。
- 提案 `c2944` / `c2960` 等：明确不保留永久双解析别名。

## Current behavior

- 旧名条目永远可读；
- 新写入只用新名；
- 没有迁移动作，所以“一次性”并未发生；
- 旧会话可以继续工作，但双解析代码永久保留。

## Options

### Option A：会话加载时一次性迁移（推荐）

- 在会话加载/resume 时扫描 leaf 分支；
- 对每条旧名条目，追加一条同 data 的新名条目（append-only，不删旧条目）；
- 读取优先新名；旧名仅用于迁移输入；
- 迁移幂等：已存在同内容新条目则不重复追加（可用 `updated_at` + data 哈希判断）。

优点：旧会话状态保留；读路径最终只认新名；符合“一次性迁移”语义。
缺点：加载路径有写副作用；需要定义失败降级（迁移失败仍可读旧名，不得中断 resume）。

### Option B：直接删除别名

- 读取只认 `"workflow"` / `"todo_checkpoint"`；
- 旧会话的 workflow/checkpoint 状态丢失。

优点：代码最简；pre-release 风险可接受。
缺点：本地已有会话可能丢失状态；用户无迁移提示。

### Option C：保留永久别名，改 spec

- 把 spec 改成“永久兼容旧名读取”；
- 优点：零代码。
- 缺点：放弃提案的“一次性”承诺；双解析永久存在。

## Recommendation

优先 **Option A**；若确认本地没有需要保留的旧会话（pre-release），可选 **Option B** 并在 release note 说明。**不建议 C**，除非明确接受永久双解析。

## Test seam

- SDK 单测：旧名条目读取 → 迁移后新名条目存在 → 再次加载不重复追加。
- 会话加载测试：迁移失败（store 只读/写入失败）时仍能读取旧名，且 resume 不中断。
- 幂等测试：同一会话多次 resume，条目数量不增长。

## Risks

- 加载路径写 session store 可能影响启动时延；应只对含旧名的会话执行。
- 迁移追加新条目会改变 leaf 分支形状；需确认 `latest_*` 选择仍正确（新条目 updated_at 更新）。
- 若其他端同时写同一会话，需要依赖 store 的锁/只追加语义。

## Open questions

1. 迁移触发点：会话加载 / resume / 首次读取？
2. 是否保留旧条目（append-only）还是删除（需要 rewrite 整个文件）？
3. pre-release 是否可以直接选 B（丢旧状态）？
