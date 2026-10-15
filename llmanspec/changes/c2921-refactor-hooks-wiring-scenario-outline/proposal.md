---
depends_on: []
title: test-hooks-wiring r1815 观察/取消矩阵收敛为场景大纲
---

## Why

r1815 规则句明文「hooks 接线验收 须以场景大纲固定『观察型 / 可取消』库缝接线」，但现行 spec 以 7 个手写 `场景:` 表达该矩阵（观察型 6 + 取消型 3 - 复用），步骤完全同构仅 event/op/context_key/reason 不同。新增一路 hooks 接线（如新事件类型）须手写场景 + binding + then 变体三处；rstest-bdd 0.6 场景大纲通路已经在 c2920（compaction 触发阈值）验证，本 change 将同一通路应用到规则句本就要求的场景，补齐既定约束而非新发明。

## What Changes

- `llmanspec/specs/test-hooks-wiring/test-hooks-wiring.feature` r1815：6 个观察型场景收敛为 `场景大纲: wiring-observe-matrix`（列 event/op/context_key，6 行）；3 个取消型收敛为 `场景大纲: wiring-cancel-matrix`（列 event/op/reason，3 行）；`unknown-op` 单例保持原样。
- `tests/bdd/steps_hooks.rs`：新增参数化 `then("hook 被调用且上下文含键 {key}")` 与 `given("注册了匹配 {event} 的 hook 且返回 block {reason}")`（均无类型提示，复用既有 `_t_hook_called`/`_t_hook_context_has_key`/`_g_hook_registered`/`_g_hook_returns`）；既有 5 then + 3 given 变体保留（agent-hooks 侧场景与组合 given 仍引用）。
- `tests/bdd/bindings_misc.rs`：该组 7 个 `#[scenario]` 收敛为 2 个 outline binding（签名参数 = 列名）。
- 跨文件影响：`agent-hooks` r1003/r1005/r1006 三场景不动（挂各自 @req、含双事件串联语义），其引用的 step 命中路径不变（老字面变体 specificity 更高，runtime `max_by(specificity)` 保证）。
- 行为合约语义不变：9 路接线的输入输出矩阵逐一保持。
