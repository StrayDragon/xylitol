---
change_id: c145-add-tool-downloader
depends_on: []
---

# c145-add-tool-downloader: 添加工具自动下载模块

## Why

pi 提供了 `tools-manager.ts`（369 行），自动下载 `fd` 和 `ripgrep` 的预编译二进制文件：
- 从 GitHub Releases 获取最新版本
- 平台感知的资产名称选择（x86_64/aarch64、darwin/linux/win32）
- tar.gz/zip 提取（多种回退方法）
- 离线模式支持
- Termux 兼容性检测

xylitol 当前假设 `fd` 和 `rg` 已在系统 PATH 中安装，缺少自动下载机制。

## What Changes

在 `src/infra/tools/downloader.rs` 下创建新模块，可选的 `infra-tools` feature：

1. `mod.rs` — 公共 API、`ensure_tool()`、`get_tool_path()`
2. `downloader.rs` — HTTP 下载、GitHub API 版本查询
3. `extract.rs` — tar.gz/zip 提取
4. `platform.rs` — 平台检测、资产名称映射

## Capabilities

- 新增 capability: `tool-downloader`

## Impact

- 新增可选 feature `infra-tools`
- 新增依赖 `flate2`、`tar`、`zip`、`reqwest`
- 约 300-400 行新 Rust 代码
