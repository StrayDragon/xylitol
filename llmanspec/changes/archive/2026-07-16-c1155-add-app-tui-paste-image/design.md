# Design — c1155-add-app-tui-paste-image（对齐 pi）

## 流程

```mermaid
flowchart LR
  CV[Ctrl+V]
  HS[HostSession]
  DR[Driver clipboard seam]
  TF[tempfile uuid]
  ED[Editor abs path]
  SUB[submit text only]
  RD[read tool]
  IM[AgentPart Image]
  CV --> HS --> DR --> TF --> ED
  ED --> SUB
  SUB -.->|model may call| RD --> IM
```

## Driver 缝

| API | 职责 |
|---|---|
| `read_clipboard_image` | → `Option<(bytes, mime)>` / Err（async spawn_blocking） |
| `write_paste_image_temp(bytes, mime)` | → `PathBuf` |
| （可选合并）`stage_clipboard_image` | 读+写 → `PathBuf` |

ScriptedDriver：注入假图字节；记录写出的 path。

## 产品 host

- keybinding `app.paste.image` 默认 `ctrl+v`
- pending 标志 → `drain_pending` 调 Driver → `insert_text_at_cursor(path)`
- 失败：`UiEntry::Error` 短行

## read → Image

- `XyTool::execute_as_parts` 默认：`[Text(execute())]`
- `ReadTool` 覆盖：图片扩展名 → text note + `AgentPart::Image`（`image_content_from_path` / resize）
- react：`tool_result` 使用 parts；`ToolExecutionEnd.result` 仍用 text 预览（跳过 base64）

## 与旧提案差异

- **砍**：提交时 resize→user Image、`@` 前缀、强制删 temp
- **加**：read 真图 + 工具 parts 出口
