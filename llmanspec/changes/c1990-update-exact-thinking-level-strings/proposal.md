---
depends_on:
- c1970-update-per-vendor-thinking-levels
---

# thinking 档名全链路精确 opaque

## Why

c1970 已把用户面档位收成**配置声明的字符串**，但请求边界仍对「内置 known 名」做 ASCII 大小写折叠 / trim（bridge `canonical_known_level`），而 `set_thinking_level`、支持集匹配、`thinking_level_map` 键仍是**精确相等**。结果：`HIGH` 可能在 Anthropic 路径被当成 `high` 预算，却过不了产品面 set；OpenAI 无 map 时又原样发 `HIGH`。这与「所见即配置、厂商字面量原样」冲突。

拍板：**方案 A — 全链路精确 opaque**（偏 c1970 精神）：不在运行时归一化档名大小写或首尾空白；内置 known 名回退也须精确匹配。

## What Changes

- **精确匹配 SSOT**：支持集成员、`set`/`cycle`、Settings 默认、`thinking_level_map` 键、以及 bridge 对「关档 / 内置 effort|budget 名」的识别，一律对**字节级精确字符串**（与配置声明相同）；MUST NOT 因仅大小写或 trim 差异而把 `High`/`HIGH`/` high ` 当成 `high`。
- **关档字面量**：产品关 thinking 的约定字面量仍为精确 `off`（既有 `THINKING_OFF`）；`OFF` / `Off` 不是关档。
- **可调判定**：支持集是否可调，以是否存在**精确非 `off`** 条目为准（不再对 `off` 做 ignore-case）。
- **Wire**：无 map 时 OpenAI 仍发当前档名原串；Anthropic 仅当档名（或 map 值）**精确**等于内置名时用内置预算，否则保持可观测失败（自由串须靠 map）。
- **非目标**：不做加载期大小写归一；不改 resume sticky 语义；不改 TUI 边框调色板的展示用模糊匹配（若保留，MUST 仅影响颜色，MUST NOT 回写档名）。

## Capabilities

- `runtime-model-registry` — set / 支持集 / 可调判定精确匹配
- `runtime-config` — map 键与声明列表精确（既有）；必要时钉死「不得归一」
- `package-ai-bridge` — resolve 取消 known-name casefold/trim
- 相关单测 / BDD（registry / bridge）

## Impact

- **Breaking（窄）**：依赖 `HIGH`/`Off` 等非精确字面量「碰巧能发请求」的配置或客户端会失败或被 set 拒绝——须改成与 `thinking_levels` 声明完全一致的字符串。
- **会话**：历史 JSONL 里若曾写入非声明大小写的 sticky 串，仍 sticky；显式 set/cycle 才进精确集。

## Ethics

- risk_level: low
- prohibited_actions: 静默把用户档名改写成小写；用 casefold 掩盖配置笔误
- required_evidence: bridge 单测证明 `HIGH` ≠ `high`；manager set 拒绝大小写变体
- refusal_contract: 不承诺上游网关接受任意大小写 effort
- escalation_policy: 若改回加载期归一（方案 B），须新 change
