# Handoff：c1905 system prompt 稳定 / 可变切分

> **临时交接**（勿升格规范）。接手人实现本 change；相邻调研结论已钉，勿重开日界争论。
> 日期：2026-08-10

## 你要做什么

按已钉 `design.md` / `tasks.md` 落地：`ContextPolicy.date_placement` 接到 `build_system_prompt`，会话钉死日历日，片段标签可测。

**不做**：StatusBar（c1895）、tool_search（c1960）、fingerprint resume 续冻、改 c1900 冻表语义。

## 为什么急：隔日会静默打爆 prompt cache

生产路径 `SystemPromptOpts.date = None` → 每次 `rebuild_system_prompt` 用 `Utc::now()` 日历日。
**隔日 resume / 跨午夜 rebuild** → system 首段字节变 → Responses 自动前缀 cache 从该点失效。

这与「稳态会话 cache 良好」不矛盾：lab 用**钉死假 date**，所以命中高。

## 相邻架构结论（2026-08-10，给判断用）

| 层 | 判断 |
|---|---|
| Message replay（`project_for_llm` → Assembler） | **合理**：历史 ToolCall 保留；错误 assistant 不回放；Env fold 钉死（c1930） |
| Provider 交互 | **合理**：c1900 冻表；`/reload` = 显式 tools epoch；`store:false` 全量重放 |
| 缓存率 | **同会话稳态良好**（下表 lab）；**全程不算优**：resume 清冻、隔日 date、MCP 仍进顶栏 `tools[]` |

### Live lab 实数（Ornith / `lab_resume_prompt_cache`）

| 步 | cache_read | input |
|---|---:|---:|
| warm1 | 0 | 426 |
| warm3 | 716 | 733 |
| resume_full（新 adapter） | 756 | 776 |

### 删工具 bust 面（闸内单测已锁）

| 变更 | Available-tools / `input` | `tools[]` |
|---|---|---|
| 仅删 MCP | **可不变**（MCP 名本不进 Available-tools） | **变 → bust** |
| 删内建 | **改写 → 前缀 bust** | **变 → 双 bust** |

Research：`llmanspec/changes/archive/2026-08-05-c1900-update-mcp-first-turn-tool-freeze/research/responses-tools-stable-id-and-resume-mcp-2026.md`（「xylitol 实证」节）。

## 已钉决策（勿重开）

见 [`design.md`](./design.md) D1–D5；一手：[`research/stable-volatile-split-2026.md`](./research/stable-volatile-split-2026.md)。

- **D1**：会话钉死日历日进 system；活时刻 → volatile（c1895）；**否决**默认日界改写 system。
- **D4**：`DatePlacement`：`SystemAsToday` | `SystemPinnedAtSession`（推荐默认）| `Omit`。
- 缺口：`date_placement` 枚举已占位，**组装层未读**。

## 关键路径

- `src/agent/prompt/system.rs`、`src/agent/context_policy/`
- 底稿：`docs/research/responses-context-layout-and-cache-2026.md`
- Lab：`cargo run -p xylitol-ai-bridge --example lab_resume_prompt_cache`
- 试验命名：根 `AGENTS.md` →「试验 / 打网命名（`lab_`）」

## 一句话

> 把「日历日」从「每次 rebuild 取今天」改成「会话钉死 + policy 可消融」；标注 stable / session_env / volatile。不要为 cache 清空合理动态内容。
