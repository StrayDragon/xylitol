---
depends_on: []
title: 裸规则压降：protocol-app 与 server-core 裸规则补可执行场景
---

# 裸规则压降：protocol-app 与 server-core 裸规则补可执行场景

## Why

specs-compact 例行压降：`llman-sdd review` 报告 4 条 unbound 裸规则（无嵌套场景的 `规则:` 块）——`protocol-app` ×1、`server-core` ×3。四者均可程序化表达（早有 `# verified-by: fn` 单测锚点），按单轨 feature-as-spec 纪律应转为 `@req:` 规则 + 嵌套可执行 `场景:`，由 BDD 步骤绑定执行，消除 review `pending` 计数。

## What Changes

- `protocol-app` r1927（线上 JSON 深解析有界）：补超限拒绝与实机深树量级放行两个场景，经 `codec::decode_str` 线入口断言。
- `server-core` r1923（首轮工具门有界决议）：补「连不上 MCP 时门时限内冻结收口」场景。
- `server-core` r1925（arm_tool_freeze 租约冲突降级）：补「写租约被持仍返回权威冻结快照」场景。
- `server-core` r1928（非产品二进制判非法信封）：补「JSON 对象/数组/纯文本 body 均 400 非法信封」场景。
- `tests/bdd/` 新增对应步骤与绑定；既有单测保留（细粒度回归与 BDD 各司其职）。

不删改任何既有 `规则:` 语义，仅裸规则 → 绑定场景的形态升级；跨 capability req_id 无冲突（`project dedupe-req-ids --dry-run` 已核）。

## Capabilities

- `protocol-app`
- `server-core`

## Impact

- 仅 `llmanspec/specs/{protocol-app,server-core}` 与 `tests/bdd/`；产品代码零改动。
- `llman-sdd review` unbound 计数归零；`cargo test --all-features --test bdd` 全绿。
