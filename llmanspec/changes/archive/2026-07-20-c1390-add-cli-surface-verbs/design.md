# Design: c1390 CLI 表面命名空间

## 定位

| 类别 | 动词 | 角色 |
|---|---|---|
| **默认** | （无子命令） | TTY → TUI |
| **surface** | `tui`, `print` | 打开某端；该端专有叶子 |
| **ops** | `resources`, `server`, `tokenizer` | 跨面管理；早退、尽量不启会话 |
| **共享 flags** | `--model` `--session` `--config` `--trust` … | 各 surface 可继承 |

对照 pi：学「默认交互 + 顶层管理 Commands」；**不**把扩展包 `install/list` 当产品主叙事；**多加**显式 surface 动词以服务多端。

## 目标 help 心智

```text
Usage: xylitol [OPTIONS] [COMMAND]

Commands:
  tui         Interactive TUI surface (default on TTY)
  print       One-shot print surface
  resources   Resource diagnostics
  server      Server lifecycle
  tokenizer   Local tokenizer cache (opt-in download)
  help        …
```

## 行为矩阵

| 调用 | 结果 |
|---|---|
| `xylitol`（TTY） | TUI |
| `xylitol tui` / `xylitol tui run` | TUI |
| `xylitol print "hi"` / `xylitol print --prompt hi` | print |
| `xylitol "hi"` / `--tui` / `--print` / 顶层 `-p` | **禁止**（未发布，不留别名债） |
| 非 TTY 裸跑 + stdin | print（管道） |
| `xylitol tokenizer …` | ops；不进 TUI |

## `tui` 子树（本波最小）

```text
xylitol tui run          # 默认叶子；可省略
# 预留（可不实现，仅设计占位，禁止空壳 MUST）：
# xylitol tui doctor     # 后置
```

本波 **SHOULD** 只实现 `tui` / `tui run`；专有诊断等后置，避免空动词。

## `print` 子树

```text
xylitol print <PROMPT>
xylitol print --prompt <TEXT>
```

MUST 非空 prompt；MUST NOT Hello! 占位。

## 模块

```text
src/app/cli/mod.rs           CliCommand::{Tui, Print, Resources, Server, Tokenizer}
src/app/cli/surface.rs       可选：TuiAction / PrintAction 解析与进入既有 run 路径
```

复用现有 `select_surface_mode` / `run_print` / `tui::run`；本 change 主要是 **argv 形状**，不是重写 bootstrap。

## 与 c1380

- `tokenizer` 已在顶层；本 change **禁止**迁入 `tui`
- depends_on c1380：保证 help 中 ops 位与合约一致

## 测试

- BDD：裸 TTY → TUI；`tui` → TUI；`print` 无 prompt 失败；`tokenizer`/`resources` 仍为顶层 Commands
- MUST NOT：顶层 `--tui` / `--print` / `-p` / `--prompt` / 位置 PROMPT
