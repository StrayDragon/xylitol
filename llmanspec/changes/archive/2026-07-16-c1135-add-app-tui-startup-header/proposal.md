---
change_id: c1135-add-app-tui-startup-header
title: "启动 header：已加载 skills / MCP（不做 prompt）"
status: draft
priority: 1135
apply_band: P3-feature
depends_on:
  - c1080-update-infra-mcp-client-product
  - c1085-update-agent-skills-runtime
author: agent
track: R
wave: editor-ux
domain: app-tui
ethics:
  risk_level: low
  prohibited_actions:
    - header 泄露完整密钥/env
    - 用 header 替代 A10 调用时注入验收
  required_evidence:
    - harness：有/无 skills、有/无 MCP 快照
    - /reload 后 header 刷新
  escalation_policy: DESIGN「无常驻多行 header」以本变更修正为 loaded-resources 槽；与 A10 正交（目录可见 ≠ 调用刷屏）
---

# c1135-add-app-tui-startup-header

## Why

pi 启动列出 Context/Skills/Prompts/…。xylitol：**不做 prompt 产品清单**；展示已加载 **skills + MCP**，便于确认 Trust/配置是否生效。

## Purpose

1. 产品 TUI 在 scrollback **上方**展示可折叠的已加载资源区（skills + MCP）
2. 启动时从 Driver 快照填充；`/reload` 成功后刷新
3. MUST NOT 列 prompt templates；MUST NOT 在 header 泄露密钥/env
4. 与 A10（`$skill` 高亮 + 静默注入）正交：本区是**目录可见性**，不是调用刷屏

## What Changes

- `Driver` 只读缝：skills 名列表 + MCP connected 摘要（配置数/已连接 id/tool_count；失败诊断短文，无密钥）
- `UiRoot` 新增 `loaded_resources` 槽（render 在 scrollback 前）
- `/reload` 后刷新该槽（与 `$skill` catalog 同路径）
- 同步 `DESIGN.md` + `design/loaded-resources.md`
- harness：有 skills / 无 MCP；有 MCP；reload 刷新
- delta：`app-tui-chrome` · `app-tui-host`

## Capabilities

- `app-tui-chrome`（modify）
- `app-tui-host`（modify）

## Out of scope

- prompt templates / extensions / packages 行
- Context 文件树清单（可后置）
- 默认展开为多行墙（默认折叠摘要）

## Impact

- 启动即可看见 skills/MCP 是否加载；reload 后同步

## Depends

- c1080 · c1085（已归档）
