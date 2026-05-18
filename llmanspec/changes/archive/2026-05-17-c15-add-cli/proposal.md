---
depends_on: [c05-init-skeleton, c10-add-config]
---

# c15-add-cli

## Why

CLI 是用户入口，需要解析命令行参数并分派到 Print/Interactive(TUI)/ACP 三种模式。这是 c05 骨架中 `src/interface/cli/` 模块的实现。

## What Changes

1. 在 `src/interface/cli/` 实现 clap derive 参数解析
2. 定义三种模式的枚举和分派逻辑
3. 集成配置加载（调用 `infra::config`）
4. `main.rs` 调用 CLI 入口

### CLI 参数结构

```
xylitol [OPTIONS] [PROMPT]

Options:
  --mode <print|interactive|acp>    运行模式（默认 print）
  --config <PATH>                   配置文件路径
  --project <PATH>                  项目根目录
  --model <ID>                      覆盖默认模型
  --yolo                            全自动模式（跳过所有确认）
  --features <LIST>                 启用的特性列表
```

### 模式分派

```rust
enum RunMode {
    Print,       // 非交互，stdout 流式输出
    Interactive, // TUI 模式（feature = "ui-tui"）
    Acp,         // ACP over stdio (feature = "infra-acp")
}
```

## Capabilities

- `cli-entry`: clap 参数解析 + 模式分派 + 配置加载

## Impact

- 新增 `clap` 依赖
- `src/interface/cli/` 和 `src/main.rs` 从占位变为实际实现
- Print 模式和 ACP 模式的分派占位（实际逻辑在 c30/c87）
