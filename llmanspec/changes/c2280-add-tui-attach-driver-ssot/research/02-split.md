# 02 TUI 与 host 的能力归属（切分观察）

> 哪段还在当前终端进程里跑。归属在 c2300。`XyDriver`（`src/app/core/driver/proto.rs`）把很多能力塞进同一个 Driver。

## 本机面

| 能力 | 干什么 | 为什么是本机面 |
|---|---|---|
| `copy_text_to_clipboard` | 复制一段历史 | 剪贴板是你坐的那台机器 |
| `read_clipboard_text` | Ctrl+V | 同上 |
| `stage_clipboard_image` | 粘贴图片 | OSC 52 也写进这个终端 |
| 键位、paint、TTY、`$EDITOR` | 面本地 | 与工作区无关 |

## 工作区 / 操作器面

`run` / `abort` / 模型 / 会话 / compact / steer / 队列 / MCP / 工具 / 工作区 bash（含 `!`） / `persist_project_trust`。`/trust` 写入的是 host 那个项目的信任文件。

reload：键位与主题贴本机面；skills / MCP / prompt 贴工作区面。

## prompt 路径

输入框回车 → `Command::Prompt` → host 推 `Event::TextDelta` → TUI 只画字。改文件走工具事件；卡住走反向 RPC。中间是 tagged JSON；换编码不换步骤。

## 灰色地带

| 事情 | 切分观察 |
|---|---|
| `export_html` / `export_jsonl` | 本机路径由 TUI 填；host 给内容、TUI 写盘。现状 Remote 把 path POST 给 host 让 host 写盘，与此相反。 |
| `import_jsonl` | TUI 读本机文件，把内容发给 host |
| `session_store()` 把 `Arc` 塞给 TUI | 进程内历史行为；Remote 已返回 `None` |

## 现状纠缠（与 HTTP 栈无关）

- Driver 上剪贴板三方法
- 人 `!` 与模型 bash 挤在同一个 `execute_bash`（见 `03`）
