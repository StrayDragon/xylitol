---
depends_on: []
skip_specs_landing: true
branch: sdd/c2750-remove-public-surface-dead-code
base_sha: aaae7f965787e2dbb6b720661c2869a57afd2120
checkpointed: true
checkpoint_sha: aaae7f965787e2dbb6b720661c2869a57afd2120
---

## Why

c2700（crate 公开面收窄）使约 40 簇原先「因对外可见而不报死码」的代码暴露为 unused/dead（约 1700 行，含各自单测）。当时为守住「仅可见性」范围以 `#[allow(dead_code)]` / 文件级 allow 暂留，本 change 是该暂留的**落地条件**：逐簇分诊并删除。

实测基线（2026-09-10，c2705–c2740 合入后）：`allow(dead_code)` 93 处 / 43 文件，其中整文件级 4 个。详见 design.md。

## What Changes

- 删除零引用簇：`agent/model/manifest.rs`（整文件）、`infra/config/value.rs`（整文件，含 InfraSecretResolver 死实现）、`infra/process/child.rs`、`agent/prompt/skill_expand.rs`、`app/server/ws.rs` 的 ServerFrame/ClientFrame、`infra/event` 第二总线方法面（EventBus::on/clear/on_lifecycle/UnsubscribeHandle）、clipboard 死路径、session manager 死方法面、settings manager 死访问器等（清单以 `rg 'allow\(dead_code\)' src/` 为准）。
- 同步删除其同文件单测与孤儿 facade 重导出。
- 分诊例外：若簇已被后续 change（c2705/c2710/c2720/c2740）激活或重写，跳过并在本 change 记录。

## 非目标

- 不新增行为、不改 wire。
- 不做 specs 压缩（另行 llman-sdd-specs-compact）。

## Capabilities

无产品 MUST；纯删除。`skip_specs_landing: true`。

## Impact

外部不可见（全部 crate 内死码）。测试面缩小。

## Further Notes

来源：c2700 apply 报告（archive/2026-09-09-c2700-refactor-crate-public-surface）。检索入口：`rg -n 'allow\(dead_code\)' src/`。
