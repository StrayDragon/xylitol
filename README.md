<div align="center">
  <img src="assets/logo.svg" alt="xylitol" width="160" height="160"/>

  # xylitol

  [![](https://img.shields.io/github/actions/workflow/status/straydragon/xylitol/ci.yaml?style=flat-square&logo=github&label=CI)](https://github.com/straydragon/xylitol/actions)
  [![](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)

</div>

---

xylitol 是一个开箱即用的个人 coding agent:以 TUI 为主要界面(另提供一次性 Print 模式),支持 OpenAI 兼容与 Anthropic Messages 两族模型 provider,内置工具执行、MCP、会话持久化与可观测性。我们刻意不做插件市场,也不为超出当前需求的扩展生态堆抽象。

> [!warning]
> **早期预览版(0.0.0-pre)**
> 配置格式、命令行行为与数据格式都可能随时变动,已知存在不稳定与 bug。欢迎试用和反馈,但请勿用于生产环境或存放不可丢失的数据。

## 截图展示

<img src="https://github.com/user-attachments/assets/4364424f-3e5e-4a41-82b2-648e8f8031ca" alt="xylitol-demo-pre-1" width="400">

## 工作原理

```
xylitol tui(客户端) ←HTTP/WebSocket→ xylitol serve(后台服务) ←→ 模型 provider / MCP / 你的项目
```

客户端只负责键盘输入、界面渲染和 TTY 这类端侧能力;模型、会话、工具与 MCP 由 host(后台服务)承担。TUI 默认连接本机正在运行的 host;`serve` 在后台常驻,关掉 TUI 不会中断会话。

## 上手

从源码构建(需要 rustup,工具链会按 `rust-toolchain.toml` 自动安装):

```bash
git clone https://github.com/straydragon/xylitol && cd xylitol
cargo build --release --locked
```

接着配置一个模型。配置文件可以放在全局(`~/.config/xylitol/config.yaml`)或项目里(`.xylitol/config.yaml`):

```bash
mkdir -p .xylitol
cp configs/example.yaml .xylitol/config.yaml
cp .xylitol/secret.env.example .xylitol/secret.env   # 填入你的 API key
```

然后编辑 `config.yaml`,取消注释 `default_model` 与至少一个 `models` 条目。API key 通过 `{{ secret.DEEPSEEK_API_KEY }}` 这样的模板从 `secret.env` 引用,不要写进会被提交的配置文件。`configs/example.yaml` 是带注释的完整样例,覆盖多模型、tokenizer、MCP 与 OTEL。

跑起来:

```bash
xylitol serve   # 后台启动 host,默认监听 127.0.0.1:18790;`serve stop` 停止
xylitol         # TTY 环境下默认进入 TUI,自动连接本机 host
```

常用参数:

```bash
xylitol tui --list-models    # 列出配置里可用的模型
xylitol tui --session <id>   # 恢复之前的会话(退出时会提示会话 id)
xylitol tui --trust          # 信任当前项目,加载它的 .xylitol/ 资源
```

首次进入某个项目时,xylitol 会询问是否信任该项目;选择不信任就会跳过该项目 `.xylitol/` 下的资源。工具本身开箱即用(allow-all),Trust 门禁只管项目本地资源是否加载,不会逐个审批工具调用。

## 开发

```bash
just setup    # 安装 prek hooks 与本地工具
just qa       # 全量门禁:fmt + clippy + 测试 + TUI harness + 文档 + 检查脚本
just qa-e2e   # 额外跑 PTY/tmux 端到端测试
```

`just qa` 中的 live-provider 网关探针在未配置 `live-provider.yaml` 时会自动跳过;CI 通过 `XYLITOL_SKIP_LIVE=1 just qa` 显式跳过。

## 文档

- [docs/architecture/](docs/architecture/) — 产品架构总览(分层、配置、会话、TUI 术语)
- [AGENTS.md](AGENTS.md) — 仓库约定与操作边界
- [llmanspec/](llmanspec/) — SDD 规格与变更历史

## 致谢

TUI 引擎(`packages/xylitol-tui`)源自 [pi-tui](https://github.com/earendil-works/pi/tree/main/packages/tui)(MIT,Copyright (c) 2025 Mario Zechner),是它的独立 fork,我们打算长期维持这个方向、不与上游对齐;provider 归因与显示名等设计参考了 [pi](https://pi.dev)。详见 [NOTICE](NOTICE) 与 [packages/xylitol-tui/NOTICE](packages/xylitol-tui/NOTICE)。

## 许可证

MIT — 见 [LICENSE](LICENSE) 与 [NOTICE](NOTICE)。
