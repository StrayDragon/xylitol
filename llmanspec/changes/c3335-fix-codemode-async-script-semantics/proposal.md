---
depends_on: []
title: "Fix codemode async script semantics"
---

# run_code 异步函数体与结果信封（对齐 pi/DSH）

## Why

会话 40eb12e7 实锤：3 次 run_code 全部返回字面 `"null"`。根因链：runner 把脚本包进**同步**块体 IIFE（顶层 await 不可用；表达式值丢弃），guest 对 undefined 序列化兜底 `"null"`——「脚本无 return」与「脚本真返 null」不可分，模型按 pi/DSH 直觉写脚本即扑空重试后放弃。先例对照（research/harness-precedent.md）：pi codemode 与 DSH PTC 均为**异步函数体 + 顶层 await/return + 显式输出通道**，无末表达式语义。

## What Changes

- run_code 脚本改异步函数体执行：顶层 `await` 与 `return` 可用。
- 结果改完成信封：completed/failed + 耗时 + 有序输出项（return / text() / console）。
- 顶层未 return 时显式「无返回值」标识，**不得**产出裸 `"null"`；用户真返 null 仍可区分。
- `text()` 升格为输出通道（pi 兼容：发射文本项，保留序列化行为）。
- 工具描述与声明提示词写明 body/await/return 语义。

## Capabilities

- agent-codemode（新增脚本执行与结果合约规则）。

## Impact

- plugins/xylitol-official-plugin-codemode（runner.rs / guest/src/lib.rs / tool.rs / declarations.rs）；BDD 与单测。

## Further Notes

- 「子工具分步结果」已达标：nested_calls 已在 details 且 TUI 逐条渲染，本单不改。
- 顶层 await 的实现路径：rquickjs 微任务泵（eval 返回 Promise 后 pump executePendingJob 至 settle），host 侧 epoch/fuel 超时取消保持不变。
