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
  → await Driver::copy_text_to_clipboard(&text)
       → InProcess: plan_clipboard_copy_async（spawn_blocking native only）
         → ClipboardCopyOutcome { pending_osc52? }
  → 若 pending_osc52：HostSession::emit_clipboard_osc52（UI 线程 Terminal::write+flush）
  → 系统块 ok / err → render_now
```

平台策略对齐 pi `clipboard.ts`（PATH 探测、不 wait wl-copy、OSC52 远程或 native 失败时）。

**TUI 竞态**：OSC 52 MUST NOT 从 blocking pool / `stdout.lock` 与差分渲染并发写出；MUST 经 host `Terminal` 在 `render_now` 之外发射。

TUI MUST NOT `use crate::infra::clipboard`。

## Slash

```text
/history-copy-last          → 复制
/history-copy-last <args>   → usage
```

无参数补全源（无子命令）。
