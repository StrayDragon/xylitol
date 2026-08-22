---
depends_on: []
---

# 去重：app-tui attach-default 与 cli-entry 双写

specs-compact 维护票：删除 `app-tui` capability 中的 `tui2`（attach-default）requirement，其语义已由 `cli-entry` 的 `ce19`（attach URL 规则，含默认 `http://127.0.0.1:18790` 与 `--attach`/`--port` 覆盖关系）与 `ce21`（未在听 fail-closed、禁静默同进程直握、print/embed 豁免）完整覆盖。行为合约零变化。

## Why

`tui2` 与 `ce21` 陈述同一可观察行为（默认 attach + 未在听非零退出），双写会在未来改动 attach 语义时产生两处需同步的 SSOT。压缩后 `app-tui` 保持「跨切面不变量索引」定位（tui3/tui4/tui5/tui-index/tui-cs 各自唯一）。

## What Changes

- 删除 `llmanspec/specs/app-tui/spec.toon` 的 `tui2` requirements 行。
- 删除 `app-tui.feature` 的 `@req:tui2` 场景及 `tests/bdd` 对应 binding/steps。
- `DEFAULT_ATTACH_URL == "http://127.0.0.1:18790"` 字面量断言迁入 `src/app/core/attach.rs` 单测（原由 BDD step 持有）。
- 其余跨 capability 相似条款（protocol-app / server-core / app-tui-host 四象限；cli-entry r69 与 app-tui-commands atm7）为分层各自义务，**不合并**。

## 非目标

任何产品行为变化；其余 spec 压缩；归档 freeze。

## Impact

`llman sdd list --specs` 中 app-tui requirements 6 → 5；BDD 场景数 -1；无代码行为差异。
