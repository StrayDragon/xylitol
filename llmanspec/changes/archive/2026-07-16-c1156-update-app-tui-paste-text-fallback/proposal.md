---
change_id: c1156-update-app-tui-paste-text-fallback
title: "Ctrl+V 无图时回退系统剪贴板文本（对齐 pi）"
status: full
priority: 1156
apply_band: P3-feature
depends_on: []
author: agent
track: R
wave: paste
domain: app-tui
ethics:
  risk_level: low
  prohibited_actions:
    - app/tui 直达 infra::clipboard（绕过 Driver）
  required_evidence:
    - harness：无图有文本 → editor 插入文本且无 Error
    - harness：无图无文本 → 短 Error
  escalation_policy: 有图仍优先路径落盘；文本仅为无图回退
---

# c1156-update-app-tui-paste-text-fallback

## Why

c1155 后 Ctrl+V 只处理图片；无图时打 `clipboard: no image`。pi 会再读系统剪贴板文本并插入 editor。用户日常 Ctrl+V 常是文本，应对齐。

## What Changes

1. infra：`read_clipboard_text` → `Option<String>`（wl-paste/xclip/…）
2. Driver：`read_clipboard_text` 缝；Scripted 可注入
3. host：`stage_clipboard_image` 为 `Ok(None)` 或失败后，再读文本；有文本则 `insert_at_cursor`，**不**报 Error；图与文本皆无才短 Error
4. 修改 ati37 合约（无图不再一律 Error）

## Capabilities

- `app-tui-input`（modify ati37）
- `infra-clipboard`（add c8）

## Out of scope

- OSC 52 读文本（远程只写不读）
- 改 bracketed paste / paste-collapse
