---
depends_on: []
skip_specs_landing: true
---

# EnvMessage 投影/裁剪/持久化/bang 收口

## Why

`EnvMessage`（bash / custom / compactionSummary / branchSummary）在 `llm_project`、session persist、cut、bang 各写一套 match。加变体要四处改，易漏「不进 LLM」或「excludeFromContext」。这是纯数据折叠，不依赖 crate 可见性，可与 c2700 并行。

## What Changes

- 在 `protocol::message`（或紧邻 helper）提供穷举辅助：`role_name` 已有；补 **LLM 投影**、**是否计入上下文**、**展示/持久化要点**，让 `llm_project` / session / bang 调 helper 而非复制 match。
- 禁止字符串 magics 判断 role。
- 不改 JSONL 字段名（camelCase wire 已稳定于会话文件；Pre-0.0.1 若要改键必须一次性迁会话，本 change **不迁**）。

## 非目标

- 不新增 Env 变体、不改 ReAct。
- 不把 helper 做成 `Xy*`。

## Capabilities

内部折叠。`skip_specs_landing: true`。

## Impact

仅 Rust 调用点。会话 JSONL 字节级不变。

## 本批依赖

无。可最先落地，给其它切片减噪音。
