# Design — c1110-add-app-tui-history-copy-last

## Busy

**允许** busy 时执行（只读复制，不改历史、不调 reload、不打断 agent）。
与多数 slash「busy 拒绝」不同；若用户要统一拒绝可后改。

## 文本来源

```text
ui_model.entries 自尾向前
  → 第一条 UiEntry::Assistant { text } 且 trim 非空
  → 否则「no assistant message to copy」
```

不包含：thinking、tool、system、未提交的 `streaming_assistant`。

## 缝

```text
PendingSlash::HistoryCopyLast
  → 选文（host/effects，读 UiModel）
  → Driver::copy_text_to_clipboard(&text)
       → infra::clipboard::copy_to_clipboard（仅 InProcess）
  → 系统块 ok / err
```

TUI MUST NOT `use crate::infra::clipboard`。

## Slash

```text
/history-copy-last          → 复制
/history-copy-last <args>   → usage
```

无参数补全源（无子命令）。
