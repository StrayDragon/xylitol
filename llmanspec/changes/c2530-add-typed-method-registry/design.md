# Design：c2530 Typed Method Registry

## D1 宏形态：声明宏优先，proc-macro 兜底

候选：

- **`macro_rules!` 声明宏（倾向）**：注册表在一个模块内以 `registry! { ... }` 单点声明，宏展开生成静态方法表、请求解析分发、handler 调用。零新 crate，契合「单 crate 逻辑分层，不拆主 crate」。局限：展开物对 rust-analyzer 跳转中等友好；复杂生成（如逐字段 struct 派生）表达力受限。
- **proc-macro attribute（兜底）**：表达力最强、派生最完整（含 OpenAPI schema），但需要独立 proc-macro crate（workspace member），违反单 crate 倾向，编译链多一环。

**决策**：先以 `macro_rules!` 落骨架 + 首个方法垂直验证（t1/t2）；仅当表达力确实不够（典型：类型化 OpenAPI schema 派生受阻）才升级 proc-macro crate。升级路径保持开放，注册表对外形状（声明语法）两个形态保持一致。

## D2 Command 归宿：保留为 port 级词表

`Command` 枚举是进程内 XyDriver 的消费词表（`dispatch.rs` 单一执行路径），不是 wire 遗留物。注册表声明**引用** Command 变体而非另造请求类型；wire 请求类型仅在 Command 覆盖不了的载荷字段处（如 subscribe 的 `cwd`）补充。禁止出现「注册表请求类型」与「Command」两套并行词表。

## D3 能力位取代代码行位

顺序敏感语义改为声明能力位，初始集合：

- `auth: readonly | writer`（写者租约准入）；
- `idem: per_rpc | bypass`（幂等准入；`bypass` 即 abort 类）;
- `resp: result | stream | job`（响应种类；job 本 change 仅预留枚举位）。

宿主执行序固化为：`bypass idem` 判定 → 幂等准入 → 租约准入 → handler，与现状行为逐一对拍。

## D4 迁移：expand-contract，全程双轨对拍

1. **expand**：注册表模块落地，方法表与 `UNARY_METHODS` 双表并存，加「两表一致」单测防漂移；
2. **migrate（分批）**：按风险从低到高分批迁移（无载荷只读 → 写类 → 特例类），每批合并后既有 BDD + 单测必须全绿；
3. **contract**：全部迁移后删除手搓转换表与 `UNARY_METHODS` 字符串表，方法表改为从注册表派生（含 404 语义与 OpenAPI 条目）。

## D5 明确不做

- job 语义实现（bash/reload 作业化）另行立项，本 change 仅预留 `resp: job` 枚举位；
- TS descriptor 发射另行立项；
- 客户端调用方法的类型化做到 `remote.rs` 现有方法一一对应为止，不重排 driver 公共 API。
