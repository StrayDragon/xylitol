---
depends_on: []
needs_specs_change: true
branch: sdd/2026-10-review-fixes
base_branch: main
base_sha: c10be4e9983564fac14b797ff9c6defaff15c4cf
---

# 线上 JSON 深解析由无界递归改显式深度上限

## Why

c2846 为深树（实机 188 层 > serde_json 默认 128）在两处开了 `disable_recursion_limit()`：`src/protocol/wire/codec.rs`（服务端解码入口）与 `src/app/core/host_client/wire_v3_client.rs`（客户端 RawOk 解析）。解析 `Value` 是递归下降——**无界深度 + 递归构建 = 栈溢出**：服务端虽另有 4 MiB 体量上限，但 4 MiB 的 `[[[[…` 足以打穿 tokio worker 栈（c2845 深树栈溢出的同族风险，只是入口换成了 JSON 轨）。本机/localhost 信任模型下风险低，但 c2845 已为深树改迭代序列化，解析侧不应留下同族无界面。

## What Changes

1. `protocol-app` 追加规则 **r1927「线上 JSON 深解析有界」**：线上输入的 JSON 深解析 MUST 设显式深度上限（远高于实机深树两个量级，覆盖 >188 层合法深树），超限按既有降级语义处理（服务端非法信封 / 客户端 RawOk→Null 退化，r1907 不断链），MUST NOT 无界递归解析线上输入。
2. 两处 `disable_recursion_limit` 增加前置深度探测（纯 std 扫描最大 `[`/`{` 嵌套深度；超限早退，不再进入递归解析）。
3. 单测：>上限构造载荷在两入口均按降级语义收口、188 层级深树照常通过。

## Capabilities

- `protocol-app`（wire 载荷解析语义；server-core 入口复用同一 codec）

## Impact / 风险

- 上限取值需同时容纳合法深树与拒绝恶意深嵌套：初值 2048（实机深树 188 的 ~10 倍、默认 128 的 16 倍），常量化并在 rule 注记。
- 深度探测为 O(n) 单趟字符扫描，载荷已被 4 MiB 上限约束，成本可忽略。
