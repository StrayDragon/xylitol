# mcp-cue

短 cue 固定 `mcp pending (see /mcp)`。agent-busy 且已有 `Next turn` 时 **不覆盖** 下轮预告。

- `/mcp` 替换 editor 槽：configured / connected / armed + tools；选中 reverse；Enter 关槽。
- 行相位 `connecting|connected|failed`。连接中可显示 `i/n`。
- 禁止把 server 名单塞进 status。任意态可开面板，开面板不 abort agent。
