---
change_id: c1050-add-cli-ai-bridge-tokenize
title: "CLI：ai-bridge tokenizer prefetch / register"
status: purpose-draft
priority: 1050
depends_on:
  - "c1030-add-package-ai-bridge"
author: agent
track: B
---

# c1050-add-cli-ai-bridge-tokenize

> **status: purpose-draft** — 待用户需要显式预热/注册 tokenizer 缓存时 promote。

## Why

c1030 要求 HuggingFace tokenizer 下载为 opt-in，避免默认静默拉大文件。需要用户可见的 CLI 来 prefetch、register 与 list，便于离线与国内镜像场景。

## Purpose

1. CLI 子命令（名称可微调）：`list` / `prefetch` / `register`（HF 或本地路径）。
2. 写入 `~/.xylitol/tokenizers/` 与用户 registry 覆盖文件。
3. 不改变 accounting 优先级；仅管理 LocalTokenizer 资产。

## Capabilities（promote 时）

- `cli-ai-bridge-tokenize`（或等价）
- modify `package-ai-bridge` registry 表面（若需）

## Out of scope

- 默认开机自动下载；footer UI

## Depends

- **c1030-add-package-ai-bridge**
