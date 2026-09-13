<div align="center">
  <img src="assets/logo.svg" alt="xylitol" width="160" height="160"/>

  # xylitol

  [![](https://img.shields.io/github/actions/workflow/status/straydragon/xylitol/ci.yaml?style=flat-square&logo=github&label=CI)](https://github.com/straydragon/xylitol/actions)
  [![](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)

</div>

---

TUI 与 Print 两种界面,OpenAI 兼容与 Anthropic Messages 两族 provider,内置工具执行、MCP、会话持久化与可观测性。不是插件市场,不为扩展生态堆抽象。

> [!warning]
> **早期预览版(0.0.0-pre)**
> 配置格式、命令行为与数据格式都可能随时变动,已知存在不稳定与 bug。欢迎试用和反馈,但请勿用于生产环境或不可丢失的数据。

## 工作原理

```
xylitol tui(端) ←HTTP/WebSocket→ xylitol serve(host) ←→ 模型 provider / MCP / 你的项目
```

端只负责键、画、TTY 这些端侧能力;模型、会话、工具、MCP 是 host 操作器角色。TUI 默认 attach 本机 host,`serve` 在后台常驻,TUI 关掉不停会话。Print 模式与库嵌入可以在同一进程内完成,不需要独立 host。

## 上手

从源码构建(需要 rustup,工具链按 `rust-toolchain.toml` 自动安装):

```bash
git clone https://github.com/straydragon/xylitol && cd xylitol
cargo build --release --locked
```

配置一个模型(全局 `~/.config/xylitol/config.yaml` 或项目 `.xylitol/config.yaml`):

```bash
mkdir -p .xylitol
cp configs/example.yaml .xylitol/config.yaml
cp .xylitol/secret.env.example .xylitol/secret.env  # 填入 API key
# 编辑 config.yaml:取消注释 default_model 与至少一个 models 条目
```

密钥通过 `{{ secret.DEEPSEEK_API_KEY }}` 这样的模板引用 `secret.env`,不要写进会被提交的 config.yaml。`configs/example.yaml` 是带注释的完整样例,覆盖多模型、tokenizer、MCP、OTEL。

跑起来:

```bash
xylitol serve                 # 后台 host,默认 127.0.0.1:18790;`serve stop` 停止
xylitol                       # TTY 下默认开 TUI,attach 本机 host
xylitol print "修复这个测试"    # 一次性 Print,适合脚本
cat error.log | xylitol print # Print 也吃 stdin 管道
```

常用参数:

```bash
xylitol tui --list-models     # 列出配置里可用的模型
xylitol tui --session <id>    # 恢复会话(退出时有提示行)
xylitol tui --trust           # 信任当前项目并加载 .xylitol/ 资源
```

首次在项目里启动会询问是否信任该项目;不信任就跳过它的 `.xylitol/` 资源。工具开箱即用(allow-all),Trust 门禁只管项目本地资源是否加载,不逐个审批工具调用。

## 开发

```bash
just setup    # prek hooks + 本地工具
just qa       # 全量门禁:fmt + clippy + 测试 + TUI harness + docs + 检查脚本
just qa-e2e   # 追加 PTY/tmux 端到端
```

`just qa` 里的 live-provider 网关探针在未配置 `live-provider.yaml` 时自动跳过;CI 用 `XYLITOL_SKIP_LIVE=1 just qa` 显式跳过。

## 文档

- [docs/architecture/](docs/architecture/) — 产品架构总览(分层、配置、会话、TUI 词汇表)
- [AGENTS.md](AGENTS.md) — 仓库约定与操作边界
- [llmanspec/](llmanspec/) — SDD 规格与变更史

## 致谢

TUI 引擎(`packages/xylitol-tui`)源自 [pi-tui](https://github.com/earendil-works/pi/tree/main/packages/tui)(MIT, Copyright (c) 2025 Mario Zechner)的独立 fork,预期持续分叉;provider 归因与显示名等设计借鉴 [pi](https://pi.dev)。详见 [NOTICE](NOTICE) 与 [packages/xylitol-tui/NOTICE](packages/xylitol-tui/NOTICE)。

## 许可证

MIT — 见 [LICENSE](LICENSE) 与 [NOTICE](NOTICE)。
