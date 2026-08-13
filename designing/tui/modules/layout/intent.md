# layout

槽高按终端行数预算。短终端仍须看见 busy `Working`，不得被 editor/footer 挤没。

- 下缘 reserved：非空队列 + toast（0|1）+ status（busy 2 行 / idle 1 行）+ footer 1 行。
- 列表槽 body 至少 1 行。loaded-resources / scrollback **不**计入 reserved。
