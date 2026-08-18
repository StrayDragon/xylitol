# TUI 留什么、server 干什么（对着现在的代码）

TUI 和 server 拆开之后，问的是：哪段代码还在你眼前这个终端进程里跑。

对照：`XyDriver` 在 `src/app/core/driver/proto.rs`。进程内实现把不少事塞进了同一个 Driver。拆开时本来就要大改。用词：[`glossary.md`](./glossary.md)。

## 两句话

你在终端里看见的、按到的、粘贴的，留在 TUI。
改仓库、调模型、跑 MCP、跑工作区里的 shell（包括你打的 `!`），放进 server。

## 一条完整路径（人打字 → 屏幕多几个字）

1. 你在输入框敲 `fix the bug`，回车。TUI **还没调模型**。它发一条 **Command::Prompt**。
2. Server 收到，开始一轮。模型每吐一点，server 就推一条 **Event::TextDelta**（例如 `"Hel"`，再 `"lo"`）。
3. TUI 只是把字追加到画面上。它不知道模型叫什么 API。
4. 模型要改文件。Server 在工作区里跑工具，再推 `ToolStart` / `ToolEnd`。TUI 画一条工具摘要。
5. 模型卡住要你选。Server 做 reverse RPC。TUI 弹出选项。你选完，TUI 发 `ApproveTool` 或 `AnswerQuestion`。Server 继续。

中间是 tagged JSON 的 wire protocol。换 encoding = 换字节格式，步骤不变。

`!ls` 按这个拆法：TUI 发 **Command::Bash**，server 在那个 cwd（或 docker 里）执行，把输出作为 Event 推回来。TUI 画命令结果。**不是**在笔记本家目录里 ls。这和旧 P6「bang 永远留 TUI 进程」相反；E 下以工作区为准。直播缺口见 [`bang-and-tools-path.md`](./bang-and-tools-path.md)。


agent 自己调的 bash 工具，本来就该在 server。现在 `execute_bash` 还兼着 bang，拆开时会拆成「用户 `!`」和「agent 工具」两条 API（UI / Esc / 会话记录 / 审批都不同），执行地点都是工作区。详见 [`bang-and-tools-path.md`](./bang-and-tools-path.md)。

## 留在 TUI

| 现在的方法 / 能力 | 干什么 | 为什么留你这边 |
|---|---|---|
| `copy_text_to_clipboard` | 复制一段历史 | 剪贴板是你坐的那台机 |
| `read_clipboard_text` | Ctrl+V | 同上 |
| `stage_clipboard_image` | 粘贴图片 | 同上；OSC 52 也得写进这个终端 |
| 键位、paint、TTY、`$EDITOR` 改输入框 | 面本地 | 没工作区什么事 |

reload：键位和主题仍是 TUI。skills / MCP / prompt 在 server。

## 在 server（操作器）

`run` / `abort` / 模型 / 会话 / compact / steer / 队列 / MCP / 工具 / **工作区 bash（含 `!`）** / `persist_project_trust` 落盘。

`/trust` 是人在 TUI 里点的，写入的是 server 那个项目的信任文件。

## 灰的

| 事情 | 怎么切 |
|---|---|
| `export_html` / `export_jsonl` | 你填的是本机路径 → server 给会话内容，TUI 写盘。要写进工作区另说 |
| `import_jsonl` | 文件在你电脑上 → TUI 读完发给 server |
| `session_store()` 把 `Arc` 塞给 TUI | 拆开后不要；Remote 已经返回 `None` |

## 搬迁时会大的

Driver 上剪贴板三方法、bang 和 `execute_bash` 缠在一起、export 路径当本机文件还是工作区文件。这些不是换个 HTTP 框架能蒙混的，是产品切分。
