<div align="center">
  <img src="assets/logo.svg" alt="xylitol" width="160" height="160"/>

  # xylitol

  [![](https://img.shields.io/github/actions/workflow/status/straydragon/xylitol/ci.yaml?style=flat-square&logo=github&label=CI)](https://github.com/straydragon/xylitol/actions)
  [![](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)

</div>

---

xylitol 是一个实验性 Coding Agent。默认界面是终端界面（TUI），另提供一次性输出模式（Print）。支持 OpenAI 兼容接口与 Anthropic Messages 接口两类模型服务。内置工具执行和 MCP（Model Context Protocol）。

> [!WARNING]
> **警告：预发布阶段（0.0.0-pre）**
> 配置格式、命令行行为和数据格式可能随时变动，已知存在不稳定和缺陷。不要用于生产环境，也不要存放不可丢失的数据。

## 截图展示

<img src="https://github.com/user-attachments/assets/4364424f-3e5e-4a41-82b2-648e8f8031ca" alt="xylitol-demo-pre-1" width="400">

## 工作原理

```text
xylitol TUI（客户端） ⇄ JSON-RPC 2.0（HTTP POST /rpc、WebSocket WS /rpc） ⇄ xylitol serve（宿主进程 host） ⇄ 模型服务商 / MCP / 项目文件
```

TUI 只负责键盘输入、界面渲染和终端（TTY）适配等端侧能力。模型、会话、工具执行、MCP 和项目信任（trust）由 host 承担。TUI 默认连接本机 host（`127.0.0.1:18790`）；host 未运行时，TUI 启动失败。关闭 TUI 不会中断 host 上的会话。Print 和库嵌入在同一进程内直接调用运行时（runtime），不经过跨进程线协议（wire protocol）。

## 快速开始

### 1. 构建

从源码构建。`rust-toolchain.toml` 固定了 nightly 工具链，rustup 会自动安装：

```bash
git clone https://github.com/straydragon/xylitol && cd xylitol
cargo build --release --locked
```

### 2. 配置模型

配置文件可以放在全局配置目录（默认 `~/.config/xylitol/config.yaml`）或项目的 `.xylitol/config.yaml`。项目配置覆盖全局配置。

```bash
mkdir -p .xylitol
cp configs/example.yaml .xylitol/config.yaml
cp configs/secret.env.example .xylitol/secret.env
```

然后按顺序修改 `.xylitol/config.yaml`：

1. 把 `models: {}` 改成 `models:`。
2. 取消注释 `default_model`。
3. 取消注释至少一个完整模型条目。

API 密钥（API key）用 `{{ secret.DEEPSEEK_API_KEY }}` 这样的模板从 `secret.env` 引用。不要把密钥写进 YAML。`configs/example.yaml` 是带注释的样例，覆盖多模型、MCP 和 OTEL。

### 3. 启动 host 与 TUI

在终端 A 启动 host：

```bash
xylitol serve
```

默认监听 `127.0.0.1:18790`。按 Ctrl-C 停止；在后台运行时，向该进程发送 SIGTERM。

在终端 B 启动 TUI：

```bash
xylitol
```

TTY 环境下，直接运行 `xylitol` 进入 TUI，并连接本机 host。

### 4. 常用命令

```bash
xylitol tui --list-models    # 列出配置里的模型
xylitol tui --session <id>   # 恢复会话；退出时会提示会话 id
xylitol tui --trust          # 本次运行信任当前项目，加载 .xylitol/ 资源
xylitol print "解释这个仓库的目录结构"   # 一次性输出，不开 TUI
```

TUI 首次进入包含 `.xylitol/` 或 `.agents/skills/` 的项目时，会询问是否信任该项目。选择不信任时，xylitol 跳过项目本地资源。工具默认允许执行（allow-all）。Trust 门禁只决定项目资源是否加载，不逐个审批工具调用。会话内可用 `/trust` 持久化信任决定。


## 开发

```bash
just setup    # 安装 Git hooks 和本地工具
just qa       # 全量门禁
just qa-e2e   # 额外跑伪终端（PTY）/tmux 端到端测试
```

`just qa` 里的真实网关探针在未配置全局配置目录下的 `dev/live-provider.yaml` 时自动跳过。持续集成（CI）用 `XYLITOL_SKIP_LIVE=1` 跳过这个探针。

## 文档

- [AGENTS.md](AGENTS.md) — 仓库约定与操作边界
- [src/AGENTS.md](src/AGENTS.md) — 代码架构单一真值源（SSOT）
- [llmanspec/](llmanspec/) — 行为驱动开发（BDD）规格与规范驱动开发（SDD）变更历史

## 致谢

TUI 引擎（`crates/xylitol-tui`）源自 [pi-tui](https://github.com/earendil-works/pi/tree/main/packages/tui)。它是独立 fork，长期不追平上游。模型服务商的来源标注和显示名等设计参考了 [pi](https://pi.dev)。详见 [NOTICE](NOTICE) 与 [crates/xylitol-tui/NOTICE](crates/xylitol-tui/NOTICE)。

## 许可证

MIT，见 [LICENSE](LICENSE) 与 [NOTICE](NOTICE)。
