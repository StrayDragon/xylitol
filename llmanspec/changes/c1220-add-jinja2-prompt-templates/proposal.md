---
depends_on:
  - c1218-remove-slash-prompt-templates
---

# c1220 — Safe minijinja system prompt + eval 可换变体（purpose-draft）

> **purpose-draft**。目录名仍为历史 `c1220-add-jinja2-prompt-templates`；升格 propose 时宜改 id（如 `refactor-agent-prompt-safe-templates`）。
>
> 探索中（2026-07-31）：主路径定为 **C**——安全 minijinja + `PromptSpec × Context → Rendered`，服务 eval A/B；**明确不**对齐 pi 的 slash prompt templates。

## Why

今日 `build_system_prompt` + `SystemPromptOpts` 字符串拼接难以：

- 稳定 dump / 同 ctx 对比两套 system 文本（eval 调优）；
- 声明式改结构而不翻 Rust；
- 与 Agent-Eval roadmap 的 **eval profile YAML** 绑定具名 scaffold。

另：live `pt5` 禁止 jinja，与「安全使用 minijinja」目标冲突，升格时须改写。

## Decisions（探索已钉）

| # | 决策 | 备注 |
|---|---|---|
| D1 | 走 **安全 minijinja**（推翻 `pt5` / `no-jinja-dep`） | 对齐 config 面 `rc23` 先例：strict + 白名单 ctx；agent 自建沙箱 Env（↛ infra） |
| D2 | **不支持** pi 式 user prompt templates（`/review` 或 `/template:name` + `$1`/`$@`） | **移除**该产品能力，非迁到 Jinja |
| D3 | Eval/测试缝：**库 `render(ctx)`**（可钉 clock）；CLI dump 非必做 | 与 D4 一致；无多 spec id |
| D4 | **单一默认**安全 minijinja 入口 + `render(ctx)`；**不**暴露多 `prompt_profile` id | 可扩展 = 类型/API 日后可加 profile；本 change 只迁现行默认组装。用户 A/B = 不同 workspace / `SYSTEM.md` |
| D5 | 不抄 pi：无 extension `before_agent_start` 链式改 prompt；无 pi-docs 段 | xylitol 非插件市场 |
| D7 | **拆 change**：先删 slash prompts 半死路径；再本线做默认组装 → minijinja | 删 prompts 可接近 quick / 小 propose；引擎 change 专做 pt5 改写 |
| D8 | **命名 A**：前置 `c1218-remove-slash-prompt-templates`；本线 **rename** `c1220` → `c1220-add-safe-minijinja-system-prompt`（propose 时改目录）；引擎 `depends_on: [c1218-…]` | 升格时执行 rename，探索阶段只记意向 |

## 影响面（D2 移除 slash prompts — 事实）

今日 xylitol 相对 pi **已半死**：

- 加载：`~/.xylitol/prompts/*.md` + 项目 `.xylitol/prompts`（`ResourceLoader`）
- 注册：`register_prompt_commands` → 命令名 `template:{name}`
- **生产展开**：c320 已删 `process_prompt`；BDD 里仅有测试辅助 `expand_template_body`
- 合约：`agent-prompt` pt3/pt4；`agent-session` a22；chrome **已禁止** header 列 templates（atc18）
- CLI：`resources` 可列出 prompts；settings 有 `prompts: []` 路径配置

移除 = 删/改写上述合约与加载/注册路径，避免「半死元数据」继续 levitation。Skills / `$skill` / SYSTEM.md **保留**（与 slash prompts 正交）。

## Purpose（升格方向）

| 优先 | 内容 |
|---|---|
| **主** | 现行默认组装 → 单一安全 minijinja + `render(ctx)`（可钉 clock）；用户 SYSTEM/APPEND 纯文本 |
| **主** | 改写 `agent-prompt`：废 pt5；新安全渲染 MUST（pt3/pt4 由**前置**删 prompts change 处理） |
| **非目标** | 多 prompt_profile；eval profile YAML；用户文件 Jinja；本 change 内删 slash prompts（见 D7） |

## What Changes（意向 · 本线 = 引擎）

- 重写 `agent/prompt` 默认组装为沙箱 minijinja；运行时走 `render(ctx)`
- 改写 pt5 → 安全 minijinja MUST；保留 SYSTEM/APPEND/context/skills 纯文本语义与 trust
- **不**在本 change 删 prompts 加载（前置 change，见 D7）

## 前置 change（D7 · 意向）

- 移除 `prompts/*.md` 发现、`register_prompt_commands` / `/template:`、pt3/pt4、a22、相关 BDD/settings 面
- Skills / `$skill` / SYSTEM.md **不动**

## Capabilities

- 本线：`agent-prompt`（主）
- 前置删 prompts：另触 `agent-session` / `runtime-resource-discovery` / settings `prompts`（视清理范围）

## Impact

- 两段交付；前置清理降低半死路径干扰后再迁引擎

## Out of scope（本线）

- 完整 SWE/Harbor 出分管线
- pi 式 extension 改 prompt
- 多 prompt_profile / eval profile YAML
- 默认分支改 live specs（须 Branch binding）

## Open Questions

1. ~~slash prompts？~~ → **D2 + D7 + D8：前置 c1218 移除**
2. ~~多 profile / eval？~~ → **D4：单一 `render(ctx)`；用户 A/B = workspace**
3. ~~SYSTEM.md Jinja？~~ → **D6：纯文本**
4. ~~同 change？~~ → **D7：拆开**
5. ~~命名？~~ → **D8：c1218-remove-… + rename c1220-add-safe-minijinja-system-prompt**

（安全白名单 ctx 字段表、嵌入模板文件布局 → propose/design 再钉，非产品分叉。）

## 探索结论（可升格）

| 线 | Change | 路径 |
|---|---|---|
| 1 | `c1218-remove-slash-prompt-templates` | 废 pt3/pt4/a22；删 loader/注册/BDD；保留 skills/SYSTEM |
| 2 | `c1220-add-safe-minijinja-system-prompt`（rename） | 废 pt5→安全 minijinja；单一默认模板 + `render(ctx)`；SYSTEM/APPEND 纯文本 |

下一步 skill：对 **c1218** 先 `llman-sdd-propose`（或确认可 `quick` 后仍建议小 propose 因改合约）；c1220 待 c1218 归档后再 propose/apply。
