---
depends_on: []
title: 修复 c2940 JSON-RPC 迁移复核发现的契约、BDD、门禁与文档问题（P0/P1）
needs_specs_change: true
branch: sdd/c2941-fix-c2940-review-followups
base_branch: main
base_sha: 46434c5dd88d8e4586e96028ee5a7d773ba16715
---

# 修复 c2940 JSON-RPC 迁移复核发现的契约、BDD、门禁与文档问题（P0/P1）

## Why

c2940 把产品线协议从 fory-v3 迁移到 JSON-RPC 2.0，主体实现成立；但复核发现 live spec/BDD 与实现之间仍有“最后一公里”：

- `AgentEnd.messages` 在 design 中声称已写入 `protocol-app` spec，实际 live spec 未写；线上丢弃该字段的语义没有契约记录。
- `protocol-app` r2918 把 `invalid_input` 列为 -32000 区间的业务错误示例，但实现/BDD 使用 -32602。
- `server-core` 仍遗留旧信封术语 `rpcId` 与版本标签 `v1`，BDD 步骤文本也沿用 `rpcId`。
- 4 MiB→413 只有 serve 单测，没有 spec 场景/BDD 绑定。
- JSON-RPC batch 无条目数上限；4 MiB body 可塞数万条请求。
- 版本命名闸的 `ALLOWED_SEGMENTS`/`EXTERNAL_NAME_MARKERS` 偏宽，且不检查 specs 中的版本标签与 `rpcId`。
- crate 图文档漏画 `kernel → serve` / `kernel → app-print`；`server-core` 导览仍写“48 规则”（实际 59）。
- 客户端/测试有少量半迁移残留（`reload_runtime` 死分支、`respond` 按载荷形状选方法、wire 往返测试用 Debug 字符串比较）。

本 change 只处理 P0/P1（契约、BDD、门禁、文档、batch 上限与少量测试清理），不处理 P2 的 resync/seq/多客户端语义缺陷（另立 `c2942`）。

## What Changes

- **P0 契约**：
  - `protocol-app` r1698 增加 `AgentEnd.messages` 面本地保留、线上不携带、transcript 以 `get_messages` 重建的语句。
  - `protocol-app` r2918 区分标准类别错误（`invalid_input`/`unregistered_method`/`internal_error` → -32602/-32601/-32603）与业务错误（-32000..-32099）。
  - `server-core` 清理 `rpcId` → `call_id`/`id`，`v1` → 本变更/首版；同步 BDD 步骤文本。
  - `protocol-app` 新增 4 MiB→413 的 BDD 场景与绑定。
- **P1 健壮性/门禁**：
  - `xylitol-protocol::codec` 增加 `MAX_BATCH_ITEMS`（100）；超限判非法请求（-32600，`data.code=batch_too_large`）；spec r2917 同步。
  - `scripts/check_no_version_names.py` 增加 `rpcId` 禁用字面量与 specs 版本标签检查（允许 URL `/vN`、会话落盘 v5/v6/v7 等外部契约上下文）。
  - crate 图补 `kernel → xylitol-serve` / `kernel → xylitol-app-print`；`server-core` 导览计数改为 59。
  - 补 4 MiB 精确边界单测。
- **P3 轻量清理（不改变产品行为）**：
  - `http_ws::unary_bound` 删除 `reload_runtime` 死分支。
  - `http_ws::respond` 改为显式方法名（调用方传 `approve_tool` / `answer_question`），不再按 `approved` 键猜。
  - wire 往返测试补 `Some(message)` / 非空 `AgentEnd` 样例与更清晰的断言。
  - BDD 的 `方法表未知为32601` 步骤改为严格断言；补 `session/resources` 不消耗 journal seq 的测试。
  - `protocol-app` r1693 的“每个 client→core 消息”改为“command-backed 方法”。
  - proposal/design/tasks 范围声明补齐 `agent-runtime`/`layer-architecture` 两个改动文件。

## Capabilities

- `protocol-app`：JSON-RPC 信封、错误码、AgentEnd 契约、batch 上限、413 场景。
- `server-core`：旧名/版本标签清理、导览计数。
- `app-tui-host` / `app-tui-bridge`：仅测试/步骤文本与客户端清理，语义不变。
- 另触及（also touched，不改语义）：`agent-runtime` / `layer-architecture` 的 BDD 绑定面与共享步骤（`tests/bdd/steps_server.rs`）随本次清理间接受影响；两个 spec 本身无需修改。

## Impact

- 行为合约变更：`AgentEnd.messages` 契约显式化；`invalid_input` 错误码说明修正；batch 上限新增；413 场景新增。
- 运行时代码：`codec.rs` batch 上限；`http_ws.rs` 死分支/respond 清理；测试。
- 不改变方法表、seq/journal、租约、幂等语义；不处理 P2 既有 resync/seq/多客户端缺陷。
- 门禁：版本命名闸新增 `rpcId` 与 specs 版本标签检查；`just qa` 必须全绿。
