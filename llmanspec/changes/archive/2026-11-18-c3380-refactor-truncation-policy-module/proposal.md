---
title: 截断策略厚模块形状与落点（wayfinder:explore）
depends_on:
  [
    c3355-refactor-sdk-inclusion-boundary,
    c3365-refactor-truncation-spec-contracts
  ]
needs_specs_change: false
branch: sdd/c3380-refactor-truncation-policy-module
base_branch: main
base_sha: 2b9cfc81e9bda5af5044d47febaeaef60488a3a0
---

# wayfinder:explore — 截断策略厚模块形状与落点

## Why

「按上限截断文本并附通知」散在 4 处实现（tools truncate.rs 行+字节双限、mcp truncate.rs、mcp resource_cache.rs 字符边界、sdk obs_gate + ports/session.rs:320）；50 KiB 数字在 tools 与 mcp 两插件各自钉死（tools/truncate.rs:2、mcp/truncate.rs:5）；通知文案两套；bash_exec/mod.rs:217 以 const _ 占位保 import。infra-mcp r80 已要求同口径，即常数统一是落实 spec 而非新决策。

## 待决策问题

1. 模块形状：模式（lines / bytes / char-boundary）、上限、通知模板、元数据（truncated_by / 计数）的接口设计。
2. 落点：依 T0 准则（sdk 或其他）；「不复用观测 IO 常量」如何与常数单点并存（r80 约束）。
3. 通知文案统一是否合约变更（依 R2 结论）；若是，spec 修订走 propose。
4. 迁移面：tools（read/grep/find/bash accumulator）、mcp（truncate/resource_cache）、runtime 侧消费者逐个换用的顺序。
5. 实施路径判定：propose 还是 quick。
6. r1140 漂移处置：spec 写「配置 max_lines 100」而实现为纯字节滚动尾窗（tools/accumulator.rs 无 max_lines）——随本票修条款还是改实现，依 c3365 调研结论定。

## 完成判据

已达成（2026-11-18）：接口形状定稿（Q1，含通知模板与常数归属）；全部消费者迁移清单（Q3 五步）；实施路径明确（独立 propose）。

## Open Questions（逐问深挖记录）

- **Q1 文案路线（已定 2026-11-18）：全统一模板（一步到位）。** bash 脚注也换通用模板，不留逐字锚行。合约变更路线：实施须走 propose 修订 r1431（infra-bash 输出截断）+ r1345 / r1346（app-tui-transcript 前缀识别与 Ctrl+O 守卫）+ 同步 BDD 步断言（steps_shared_thens.rs:132-143 现断言 `[Full output:` 与 `lines shown`）+ TUI 识别逻辑。sdk 侧只做引擎 + 单一通用通知模板；bash 溢出临时文件机制不动（r1431 的溢出语义保留，仅人读脚注形状变更）。

## Open Questions（逐问深挖记录）

- **Q1 模块形状与文案路线（已定 2026-11-18）：组合式。** sdk `truncation` 纯函数模块零新依赖：引擎 `truncate(text, Options { mode: Lines|Bytes|Both, limit_lines, limit_bytes, direction: Head|Tail }) -> TruncationResult { content, truncated, truncated_by, kept_bytes, dropped_bytes, limit, direction }`（字符边界安全内建）；通知两类——bash 逐字脚注渲染器留 tools 插件（r1431 三层钉死，零 spec 变更），mcp/grep/find/read 统一 `generic_bytes_notice(kept, dropped, limit)`（采用 mcp 现格式为模板，语义已钉死，措辞换新须同步 4 处 BDD 步断言）；常数 `DEFAULT_OUTPUT_TRUNCATION_BYTES = 50 KiB` 为工具输出唯一口径，obs 与 codemode 各持独立常数（r2934 隔离）。bash 溢出临时文件逻辑留 tools（infra），不入 sdk。
- **Q2 r1140 漂移方向（已定 2026-11-18）：修 spec 对齐纯字节滚动尾窗。** r1140 删「max_lines 100」句，主语去类型化（OutputAccumulator → 流式累积器），上限表述对齐「同一口径」；随实施直接编辑 live feature（免 change 生命周期），场景不动。理由：字节是护上下文的真约束（超长单行场景行数上限不设防）；tail 窗 + 溢出文件 + r1431 脚注已闭环；补行数上限要为不存在的需求付配置面/双限交互/BDD 三份成本。
- **Q3 常数归属 + 迁移顺序 + 路径（已定 2026-11-18，打包确认）：** 常数——sdk DEFAULT_OUTPUT_TRUNCATION_BYTES = 50 KiB 为 read/grep/find 与 mcp 唯一口径（r2934「同一口径」落实）；obs_gate 与 codemode（100 KiB）独立常数不动；resource_cache 256 KiB（r3278）复用引擎 Bytes 模式、上限自持。迁移顺序——①sdk truncation 模块 + 单测（含字符边界）②mcp truncate + resource_cache 换引擎（通知即模板原样，用户可见零变化）③tools read/grep/find 换引擎 + 通用通知（用户可见文案变化，同步 BDD 步断言）④tools bash 不动（滚动窗 + 溢出是流式 infra，与整段截断引擎两种动物）⑤r1140 直接编辑 + 全量门禁。路径——独立 propose（与其他三票零共享文件；无行为合约变更，spec 侧仅 r1140 直接编辑），最终是否并入大 propose 由 T5 裁。

## Further Notes

（决策与 research 结论回写处）

## 勘误（2026-11-18，归档后复核；正文保留原样）

1. **本文存在两个重复的「## Open Questions（逐问深挖记录）」节，Q1 有两版结论**：第一节（「全统一模板（一步到位）」）为同日早前草稿，已被第二节的「组合式」结论取代。有效决策以第二节为准（sdk truncation 引擎 + `generic_bytes_notice`，bash 逐字脚注渲染器留 tools），并经 c3350 wayfinder 地图 Decisions so far 采认；勿引用第一节。
