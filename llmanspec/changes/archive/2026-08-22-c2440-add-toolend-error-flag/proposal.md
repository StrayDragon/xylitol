---
depends_on:
- c2425-add-timeout-bounds
skip_specs_landing: true
branch: sdd/c2440-add-toolend-error-flag
base_sha: 637fc6eb3888bbae78c21d169b65cb056d9be41c
checkpointed: true
checkpoint_sha: 637fc6eb3888bbae78c21d169b65cb056d9be41c
---

# ToolEnd 下行携带 is_error 失败标记

手动验收发现：attach 模式下超时终止的工具行不显示失败态（轨色非红、错误文本被流式输出吞掉）。根因是 wire 映射丢弃了 `XyEvent::ToolExecutionEnd.is_error`——序列化侧 `..` 忽略、反序列化硬编码 false。

## 处置说明（票据补记）

修复提交 `e732d5a1` 因流程疏漏先行落在 main；本票为**补记收编**：`skip_specs_landing: true`（合约行 pa-err1 已随修复在 main 落地），工件记录根因/裁决/验证，生命周期自此恢复正常追踪。后续同类 wire 行为变更 MUST 先 start 再落。

## What Changes（已实施）

- `Event::ToolEnd` 增加 `#[serde(default)] is_error: bool`；to/from wire 双向携带。
- 旧载荷缺字段按成功解析（journal 兼容）。
- bindings.ts 经 gen-sdk 重生（tool_end 增 `is_error?: boolean`）。
- TUI bridge：is_error 且非 MCP 且输出未含错误文本时追加失败行（流式输出不再吞错）。
- protocol-app 新增 **pa-err1** 钉住该下行合约。

## 非目标

journal 历史回放补标（旧行按成功，与既有冷恢复语义一致）；print 面（同进程已正确）。

## Impact

attach 客户端工具失败获得红色轨 + 可见错误行；wire 为加字段演进，旧客户端忽略即可。
