---
change_id: c1045-add-ai-bridge-gguf-tokenize
title: "ai-bridge：从本地 GGUF 提取 tokenizer"
status: purpose-draft
priority: 1045
depends_on:
  - "c1030-add-package-ai-bridge"
author: agent
track: B
---

# c1045-add-ai-bridge-gguf-tokenize

> **status: purpose-draft** — 待本地 llamacpp/GGUF 成为主路径且缺少 HF tokenizer.json 时 promote。

## Why

c1030 LocalTokenizer 支持 Builtin 与 HuggingFace `tokenizer.json`。纯本地 GGUF 部署可能无单独 tokenizer 文件，却内嵌 tokenizer metadata；需要从 GGUF 提取并接入 registry。

## Purpose

1. registry 增加 `TokenizerSource::Gguf { path }`（或等价）。
2. 从 GGUF 解析 tokenizer 元数据并加载为可 `count` 的 LocalTokenizer。
3. 失败时按 c1030 链降级到 Heuristic，provenance 诚实。

## Capabilities（promote 时）

- modify `package-ai-bridge` / `package-ai-bridge-accounting`

## Out of scope

- 模型推理本身；静默下载 GGUF

## Depends

- **c1030-add-package-ai-bridge**
