---
title: 截断与通知文案 spec 合约全集（wayfinder:research）
depends_on: []
needs_specs_change: false
branch: sdd/c3365-refactor-truncation-spec-contracts
base_branch: main
base_sha: 51da33145fac0615f3149eb7a430fb66084541c1
---

# wayfinder:research — 截断与通知文案 spec 合约全集

## Why

T3（截断策略厚模块）需要先知道合约把哪些差异面钉死了。已知：infra-mcp.feature:80 要求「上限须与既有内置工具截断同一口径（50 KiB），不得复用观测 IO 常量」；agent-tools.feature:469 钉 bash 截断视图；tools 与 mcp 两插件通知文案两套（"[.. limit reached]" vs "[truncated: .. kept, .. dropped]"）。

## 查证问题

1. 全 specs 扫截断/上限/通知文案条款：infra-mcp、agent-tools（truncate_output、read/grep/find 上限）、agent-runtime、app-tui-* 呈现面，逐条引用原文。
2. 两套通知文案是否各自被 spec 钉死（引用条款）；统一文案是否构成合约变更。
3. sdk obs_gate.rs 常数与 ports/session.rs:320 的截断语义；「不得复用观测 IO 常量」的边界具体指什么。
4. 汇总：统一截断策略模块必须保留的差异面清单（合约钉死 vs 可统一）。

## 产出

结论写入 llmanspec/changes/c3365-refactor-truncation-spec-contracts/research/，全部断言带条款引用（capability:行号）；摘要回写本 proposal Further Notes。

## Further Notes

调研完成（2026-11-18，结论全文见 research/truncation-spec-contracts.md；主会话独立复核 r1431 逐字钉死、BDD 步断言、r1345/r1346 前缀识别与 r1140 漂移属实）。要点：

- 统一文案判定：bash 脚注 `[Full output: <path>. Truncated: <N> lines shown (<limit> limit)]` 被三层钉死——spec 逐字（infra-bash r1431，含 (unavailable) 回退与未截断不追加）＋ BDD 步断言 `[Full output:` 与 `lines shown`（tests/bdd/steps_shared_thens.rs:132-143）＋ TUI 前缀识别（app-tui-transcript r1345/r1346）；改形状即合约变更。mcp 侧仅语义钉死（infra-mcp r2934 要求 kept/dropped bytes + limit 三要素），改字串非合约变更但须保三要素。两形状不相交，「单一同一字符串」无 spec 变更做不到；可行 = 组合式（保留 bash 逐字锚行 + 追加通用 bytes 标注行），或全收敛走 propose 改 r1431/r1345。
- 必须保留的差异面（合约钉死，c3380 不得改变）：bash 尾窗 + 逐字脚注 + JSON 契约（agent-tools r1147）+ UTF-8 边界安全；read 2000 行 / 50KB 双限与超 10MB 拒绝（r1136 / r4）；mcp 50KiB、不得复用观测 IO 常数、多分片保真、错误与非 MCP 豁免、有 servers 才装配（r2934）；资源缓存 256KiB 独立（r3278）；拒绝族（acceptance 4000 / todo 80）不得改截断；bash 尾窗 vs read/grep/find 头窗方向 spec 未钉、r42 pi 对齐需评估。
- 可统一面：mcp/grep/find/read notice 措辞（ls/read 改动须同步 BDD 步断言）、默认 limit（ls 200 / grep 100 / find 1000）、grep 单行 500 chars、codemode 100KiB、tools 与 mcp 两处 50KiB 常数合一。
- 漂移观察（移交 c3380 处置）：agent-tools r1140 写「配置 max_lines 100」，实现 OutputAccumulator 为纯字节滚动尾窗、无 max_lines 配置（tools/accumulator.rs:9-19；max_lines 仅在 TruncationOptions 供 read/grep/find 使用）。
