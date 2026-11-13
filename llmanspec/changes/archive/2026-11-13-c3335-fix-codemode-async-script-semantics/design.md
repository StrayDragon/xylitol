# Design — run_code 异步函数体与结果信封

## 0. 现状复核（2026-11-13，HEAD c778827；证据链见 research/harness-precedent.md）

- runner.rs:186 包装 `const __result = (() => { <code> })();`：同步块体——return 可用、顶层 await **不可用**、表达式值丢弃。
- guest/src/lib.rs:103-110：`json_stringify(undefined)` → None → 兜底 `"null"`——无 return 与真 null 不可分。
- text()（preamble）仅序列化返回，非输出通道；pi 同名为发射通道——模型直觉错位。
- 触发会话：3 次表达式脚本全部 `"null"`，模型放弃（会话 log/000001.jsonl entry 9-15）。
- pi/DSH 先例一致：异步函数体 + 顶层 await/return + 显式通道；无末表达式语义。

## 1. 目标与边界

- 目标：脚本语义与结果合约对齐 pi/DSH（见 §2）。
- 边界：不引入末表达式 REPL 语义；ALL_TOOLS/searchTools/describeTool 不变；nested_calls 分步呈现已达标不改；限额（fuel/epoch/内存/max_result_bytes）与错误分类（r3254）不变。

## 2. 设计

1. **异步函数体**：脚本包装改 `const __result = await (async () => { <code> })();`。guest eval 得 Promise 后**微任务泵**至 settle（rquickjs executePendingJob 循环 + settle 标志），再取完成值；宿主侧 epoch/fuel 超时取消路径不变（泵循环内长任务仍被 epoch 打断）。await 挂起的 host 工具调用仍按现同步 import 语义执行。
2. **完成信封**：结果文本首行 `Script completed in {ms}ms` 或 `Script failed: {error}`；随后有序输出项：
   - 顶层 return 值（defined 时）一项，JSON 序列化；用户真返 null 渲染为 `return: null`，可区分。
   - text() 每次调用一项（按调用序）。
   - console 调用一个 `<console_output>` 块。
   - 无任何输出项时显式一行 `(no output — no top-level return, no text()/console calls)`。**禁止**裸 `"null"` 作为整个结果。
   - 截断语义保留（max_result_bytes / token 上限），截断时信封首行仍在。
3. **text() 升格输出通道**：preamble 改为 `function text(v) { __emit(v) ; return 序列化值 }`（经新增 host import `__emit` 或复用 __log 通道加 kind 区分）；序列化行为不变（string 透传、其余 JSON）。pi 兼容：模型调用 text() 即见输出。
4. **描述与提示词**：tool.rs 与 declarations.rs 写明「code is the body of an async function; top-level await and return work; output channels are return / text() / console.log」。

## 3. spec 落点（agent-codemode）

新增规则（中文、产品可观察行为）：

`规则: script-body-and-result` —— run_code 脚本 须以异步函数体执行（顶层 await 与 return 可用）；结果 须为完成信封（completed/failed + 耗时 + 有序输出项：return、text()、console）；顶层未 return 且无输出通道调用时 须显式无输出标识，不得产出裸 null；text() 须为输出通道。

场景（3）：
- 场景: 顶层await与return可用 —— 当 脚本顶层 await 调用工具并 return 汇总值 那么 信封为 completed 且含 return 项。
- 场景: 未return不产出裸null —— 当 脚本无 return 且无 text()/console 那么 结果含显式无输出标识且非裸 null。
- 场景: text发射输出项 —— 当 脚本调用 text(值) 那么 信封含该文本项。

## 4. 测试边界

- 单测：guest 泵循环（await 工具调用后 settle）、undefined/真 null/字符串三种完成值信封、text() 发射、截断保信封。
- BDD：agent-codemode 新场景 + r3253/r3254 回归（嵌套 dispatch、限额错误分类不变）。
