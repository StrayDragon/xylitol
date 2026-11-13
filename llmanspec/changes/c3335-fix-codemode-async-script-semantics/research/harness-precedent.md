# 先例对照：run_code 脚本语义（pi codemode / DSH PTC / xylitol 现状）

调研日期：2026-11-13。触发会话：40eb12e7-32ff-4195-b060-8b9dac6163ff
（3 次 run_code 全部返回字面 "null"，模型放弃；日志记录完整，非记录丢失）。

## pi codemode（https://pi.dev/docs/latest/codemode）

- 原文：「It runs as the body of an async function in a QuickJS sandbox, so top-level await and return work.」
- 多通道输出：text() 每次发一条文本项；顶层 return「adds the value like text()」；console 调用收进 <console_output> 块；另有 image()/exit()/store-load。
- 结果信封恒存在：「Script completed / Script failed」+ 耗时 + 有序输出项；多项时 ==> text N/M <== 头；失败保留部分输出。
- 无末表达式 REPL 语义。
- ALL_TOOLS = 全部可调用工具 {name, description}。

## DSH PTC（本机 checkout：@deepseek-ai/dsh-tool-executor-ptc、dsh-ptc-runtime-node）

- programFormat = "function-body"：「the BODY of an async function (top-level await and return work)」。
- 双通道：return（completion value，须无损 JSON，否则显式失败）+ console.log；undefined 编码为 {}，可区分（process.js prepareCompletion）。
- 社区兼容层 pi-codemode-guard 对照表：「console.log(...) and return are PTC's output channels and are Pi-compatible.」

## xylitol 现状（HEAD c778827）

- runner.rs:186 包装为同步 IIFE：const __result = (() => { <code> })();——return 可用（对齐），顶层 await 不可用（偏差）。
- 表达式脚本 → __result=undefined → guest json_stringify(None) 兜底 "null"（guest/src/lib.rs:110）——与用户真返 null 不可分，模型困惑（触发会话实锤）。
- text() 仅为序列化帮助函数（preamble），非输出通道——与 pi 同名不同义，误导 pi 直觉的模型。
- nested_calls 已在 details 且 TUI 已逐条渲染（bridge/mod.rs:321 起）——「分步结果/子工具」已达标，无需改动。

## 结论

- 语义基线 = 异步函数体 + 显式 return（pi/DSH 一致）；不引入末表达式 REPL 语义。
- 修复面：async 包装（guest 微任务泵）、完成信封 + undefined 显式化、text() 升格输出通道、工具描述写明 body 语义。
