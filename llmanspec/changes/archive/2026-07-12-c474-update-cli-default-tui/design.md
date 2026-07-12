# design — c474 CLI default TUI

## 分发真值表（TTY）

| 调用 | 模式 |
|---|---|
| `xylitol` | TUI |
| `xylitol --tui` | TUI（显式，冗余但保留） |
| `xylitol "fix this"` | print one-shot |
| `xylitol --prompt "fix this"` | print one-shot |
| `xylitol --print --prompt "x"` | print |
| `xylitol --tui --prompt "x"` | TUI（`--tui` 胜出；MVP 不自动提交 prompt） |
| `xylitol --list-models` | 列表（非 TUI） |

## 非 TTY

- 无 prompt：错误退出（提示用 `--prompt` 或在 TTY 下启动），**MUST NOT** `"Hello!"`。
- 有 prompt / stdin 管道 + `--print`：print。

## 实现要点

- clap：`--prompt` / `-p` + 位置 `PROMPT` 合并为 `one_shot_prompt()`。
- `want_tui` 优先于 print。
