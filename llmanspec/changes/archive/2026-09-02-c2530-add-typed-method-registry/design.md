# Design：c2530 Typed Method Registry

## D1 注册表形态：声明式 const 表 + strum 穷举守卫（as-built）

候选评估（实现期调研，三形态保障矩阵）：macro_rules! / 普通 const 表 / UnaryMethod 枚举。

- 宏的编译期锚定只能断言变体**存在**（`matches!` 模式），读不到 serde attr，锚不了「方法名 = serde tag」的相等性；运行时 tag 测试走真实 serde 路径（unknown-variant / alias / 大小写），是更强的等价保障。
- tag-injection 解析是全方法统一路径，宏没有逐方法可生成的工件——宏只剩仪式成本（展开物可导航性差、违反「能内联不套 wrapper」）。
- **穷举性缺口（宏与 const 表共有的洞）**：新增 Command 变体而漏注册行时，编译与 BDD 全绿、方法悄悄不可达。以 `strum::VariantNames`（仓库既有依赖）覆盖测试关闭：非 wire 变体（ApproveTool / AnswerQuestion / Quit）显式豁免，其余 34 个 wire 变体逐一断言存在注册行。
- UnaryMethod 枚举形态否决：边界入口全是字符串（路径参数、信封 method、幂等键），枚举只增加第三套词表。

**决策（as-built）**：普通 const 表（结构体字面量行，rust-analyzer 直跳）；`is_unary_method` / OpenAPI 条目从表派生；穷举性由 VariantNames 覆盖测试守卫。D1 原文「宏驱动」就此修订——宏的两个卖点分别被更强的运行时测试与 const 表自身覆盖。

## D2 Command 归宿：保留为 port 级词表

`Command` 枚举是进程内 XyDriver 的消费词表（`dispatch.rs` 单一执行路径），不是 wire 遗留物。注册表声明**引用** Command 变体而非另造请求类型；wire 请求类型仅在 Command 覆盖不了的载荷字段处（如 subscribe 的 `cwd`）补充。禁止出现「注册表请求类型」与「Command」两套并行词表。

**关于客户端大量 `id: None`**：`Command.id` 是传输级关联位（dispatch 忽略它，由调用方提取/回显）。两条路径各有关联机制——远端 HTTP 路径关联走**信封 rpcId**（`HttpWsClient` 每调用铸 uuid，兼幂等键 c2460），Command.id 冗余；进程内路径无信封，才用 Command.id 关联 `Response { id }`。远端客户端如实填 `None`（与手搓表行为一致，wire 解析侧也剥离载荷 id）。

## D3 能力位取代代码行位（as-built 修订）

顺序敏感语义改为声明能力位，初始集合：

- `auth: readonly | writer`（写者租约准入）；
- `idem: per_rpc | bypass`——bypass 为**结构性** pre-admit 特例，现集 = {host.describe, reload, loaded_resources}，守卫测试锁定；
- `resp: result | stream | job`（响应种类；job 本 change 仅预留枚举位）。

**对提案原表述的修正**：原文写「abort 旁路幂等声明化」。代码现实：abort 正常走 admission + 租约（可幂等重放，行为正确）；仅 **reload 窗口内**的 abort 走 pre-admit 短路——那是 `host.abort_reload()` 运行时状态依赖的条件分支，静态能力位不可表达，保留为显式特例。注册表如实声明 `abort: idem=per_rpc`，宿主执行序固化为：bypass 判定 → 幂等准入 → 租约准入 → handler，与现状逐一对拍。

## D4 迁移：expand-contract，全程双轨对拍

1. **expand**：注册表模块落地，方法表与 `UNARY_METHODS` 双表并存，加「两表一致」单测防漂移；
2. **migrate（分批）**：按风险从低到高分批迁移（无载荷只读 → 写类 → 特例类），每批合并后既有 BDD + 单测必须全绿；
3. **contract**：全部迁移后删除手搓转换表与 `UNARY_METHODS` 字符串表，方法表改为从注册表派生（含 404 语义与 OpenAPI 条目）。

## D5 明确不做

- job 语义实现（bash/reload 作业化）另行立项，本 change 仅预留 `resp: job` 枚举位；
- TS descriptor 发射另行立项；
- 客户端调用方法的类型化做到 `remote.rs` 现有方法一一对应为止，不重排 driver 公共 API。
