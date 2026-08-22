---
depends_on: [c2425-add-timeout-bounds]
---

# 工具 timeout 模型可请求化（钳制到全局上界）

c2425 的后续精化：模型（LLM）MAY 在调用 bash/grep/find 时显式传 `timeout` 秒数来表达「这个任务最长需要 N 分钟」（例如 10 分钟长构建）；程序保留最高权威——超过全局兜底上界（600 秒）的请求被**钳制**而非拒绝，计时器必然武装。

## Why

c2425 落地时显式值 >120s 会报 AboveMax 错误，模型无法表达合法长任务。用户裁决：允许模型设置到全局上界；程序只做钳制，不做浪费一轮的硬拒。

## What Changes

- `MAX_TOOL_TIMEOUT_SECS` 120 → **600**（= `MAX_EXTERNAL_WAIT_SECS`）。
- `from_i64_opt` / `from_i64`：移除 AboveMax 拒绝臂——正整数一律接受，由新增 `.clamped()` 钳到上界；0/负数仍拒绝。
- 工具链改为 `from_i64_opt(..)?.or_default(工具默认).clamped()`；schema 文案改「max 600」。
- specs 同句：agent-tools t24（可请求 + 钳制语义）、r6 措辞对齐；t29/fs 工具不变。

## 非目标

fs 四工具有界策略；per-tool 上界差异（统一全局 600）；config 化。

## Impact

长任务可表达；AboveMax 错误路径消失（相关单测改为钳制断言）；BDD negative/zero 场景不受影响。
