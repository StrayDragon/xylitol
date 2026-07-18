# Tasks: c1340

## Specs

- [x] live `app-tui-transcript`：att14 Tail；att16 超限 tool/bash 禁展开
- [x] attach change

## Implement

- [x] write：`TruncateFrom::Tail` + earlier hint
- [x] `hard_truncated` 检测；tool/bash 强制尾视口 + disabled hint
- [x] ToolExecutionEnd：截断时用 combined 替换流式缓冲
- [x] 单测：write earlier；truncated bash 无全文展开

## Verify

- [x] `cargo test -q --lib app::tui`
- [x] `llman sdd validate c1340… --strict --no-check`
