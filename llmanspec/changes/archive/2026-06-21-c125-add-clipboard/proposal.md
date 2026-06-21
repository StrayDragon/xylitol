---
change_id: c125-add-clipboard
depends_on: []
---

# c125-add-clipboard: 添加剪贴板模块

## Why

pi 提供了一套完整的剪贴板操作能力（`clipboard.ts` + `clipboard-image.ts` + `clipboard-native.ts`，共约 555 行），支持：

- 通过平台原生工具复制文本到系统剪贴板（pbcopy、clip、wl-copy、xclip、xsel、termux-clipboard-set）
- 通过 OSC 52 转义序列支持远程会话（SSH）的剪贴板操作
- 从剪贴板读取图片（wl-paste、xclip、macOS 系统剪贴板、PowerShell）
- 图片 MIME 类型检测与格式处理

xylitol 当前完全没有剪贴板模块，无法支持 TUI 模式中的复制/粘贴交互。

## What Changes

在 `src/infra/clipboard/` 下创建新模块：

1. `mod.rs` — 模块入口，导出公共 API
2. `native.rs` — 平台原生剪贴板工具封装
3. `osc52.rs` — OSC 52 终端转义序列支持
4. `image.rs` — 图片剪贴板读取支持

## Capabilities

- 新增 capability: `clipboard`

## Impact

- 新增可选 feature `infra-clipboard`
- 无向后兼容性问题（全新模块）
- 约 400-600 行新 Rust 代码
