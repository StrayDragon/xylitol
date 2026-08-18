# 窗口管理器用户怎么拉起 `xylitol serve`

> 不实现。用户画像：习惯用 **窗口管理器**、经常 **多开终端** 的人（例如 i3 / sway / Hyprland，或 tmux 里多窗格）。正文不绑定某一个 WM。

`xylitol serve` 是显式起的 host（proposal P3），不是开第一扇 TUI 时偷偷拉起来。

常见习惯（任选，产品不必内置）：

| 习惯 | 做什么 |
|---|---|
| 图形会话 autostart | 登录后起一次 serve（xdg autostart、systemd `--user`、WM 自己的 startup 列表） |
| 工作区第一条终端 | 进某个 workspace 时手动或脚本先 `xylitol serve`，再开若干 `xylitol tui --attach` |
| 终端复用器 | tmux/zellij 里一个 pane 跑 serve，其余 pane attach |

关终端窗口不得杀掉 serve（P5）。serve 进 Docker 当可选沙盒时，autostart 的是 `docker run … xylitol serve`（或 compose），TUI 仍在宿主机连 published port。

本机 Linux 无 Docker 时，serve 可以 `bind` 一个 UDS 路径；多扇 TUI 都 `connect` 同一路径。例子见 [`glossary.md`](./glossary.md)「谁 bind、谁 connect」。
