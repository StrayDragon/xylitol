---
depends_on: []
---

# c05-init-skeleton

## Why

项目当前是单一 crate 占位（`xylitol v0.0.0-dev`），需要建立领域分层的模块结构和 feature flags 体系，为后续所有功能模块提供清晰的物理边界和编译时裁剪能力。这是所有后续 change 的基础。

### 设计决策：单 crate vs 9-crate workspace

PRD §0.9 推荐了 9-crate workspace（pi-cli, pi-core, pi-tools, pi-tui, pi-lsp, pi-dap, pi-session, pi-config, pi-test-support）。经权衡，选择**单 crate + 领域模块 + feature flags** 方案：

- **分发简单**：单静态二进制，无需处理 crate 间版本协调
- **feature flags 天然可插拔**：编译时裁剪，不用的功能零开销
- **开发迭代快**：无 workspace 依赖解析开销，改一处即编译
- **依赖管理简单**：无需处理 crate 间的版本对齐和发布顺序

PRD §13.10 的测试目录规范需适配为单 crate 结构（`tests/support/` 替代 `pi-test-support/` crate）。

## What Changes

1. 重构 `src/` 为三层领域模块：`agent/`（核心领域）、`infra/`（基础设施）、`interface/`（用户接口）
2. 定义 feature flags：每个可选功能对应一个 feature，default 仅含核心
3. 各层放置占位 `mod.rs` 入口，编译通过但无实际逻辑
4. 删除原 `src/lib.rs` 占位函数，改为三层模块声明
5. `configs/` 目录 + 空 `config.schema.json` 占位

### 目录结构

标注说明：`(built-in)` 始终编译，通过 config 层控制；`(feature-gated)` 通过 Cargo feature flag 控制。

```
xylitol/
├── Cargo.toml                  # 单 package + features
├── src/
│   ├── main.rs                 # CLI 入口点
│   ├── lib.rs                  # 三层模块声明 + re-export
│   ├── agent/                  # 核心领域
│   │   ├── mod.rs
│   │   ├── loop.rs             # agent 执行循环（占位）
│   │   ├── planner.rs          # 规划-执行分离（占位, feature = "agent-planning"）
│   │   ├── model.rs            # 模型注册与锁（占位）
│   │   └── tools/              # 内置工具集 (built-in)
│   │       └── mod.rs
│   ├── infra/                  # 基础设施
│   │   ├── mod.rs
│   │   ├── config/             # YAML 配置系统
│   │   │   └── mod.rs
│   │   ├── lsp/                # LSP 集成（feature = "infra-lsp"）
│   │   │   └── mod.rs
│   │   ├── dap/                # DAP 集成（feature = "infra-dap"）
│   │   │   └── mod.rs
│   │   ├── session/            # 会话快照（feature = "infra-session"）
│   │   │   └── mod.rs
│   │   ├── hooks/              # Hook 事件系统 (built-in)
│   │   │   └── mod.rs
│   │   ├── security/           # 安全策略 (built-in)
│   │   │   └── mod.rs
│   │   └── skills/             # Skills & MCP（feature = "infra-skills"）
│   │       └── mod.rs
│   └── interface/              # 用户接口
│       ├── mod.rs
│       ├── cli/                # clap 参数解析
│       │   └── mod.rs
│       ├── tui/                # ratatui TUI（feature = "ui-tui"）
│       │   └── mod.rs
│       ├── print.rs            # Print 模式 (built-in)
│       └── acp.rs              # ACP 模式 (feature = "infra-acp")
├── configs/
│   └── config.schema.json      # 占位
└── tests/
```

### Feature Flags（两层启用策略）

两层启用策略：
- **始终编译（built-in）**：无 feature flag，始终编译进二进制，通过 `config.yaml` 运行时控制启用/禁用
- **可选编译（feature-flagged）**：Cargo feature flag 控制，引入重量级依赖或非必需集成

判定为始终编译的标准：轻量无重依赖、对 agent 正确运行至关重要、集成点深入（feature-gating 需大量 `#[cfg]`）

```toml
[features]
default = ["ui-tui", "infra-session", "ui-review"]

# ── agent/ layer ──────────────────────────────
agent-planning = []      # 规划-执行分离（Architect/Editor/Validator）
agent-model-lock = []    # 模型抢占锁（Phase 2: 仅本地模型需要）

# ── infra/ layer ──────────────────────────────
infra-lsp = []           # LSP 集成层（lspz agent-sdk）
infra-dap = []           # DAP 集成层（dapz -- Phase 2 placeholder）
infra-acp = []            # ACP Agent Client Protocol（agent-client-protocol SDK）
infra-skills = []        # Skills & MCP 支持（rmcp via adk-tool）
infra-session = []       # Session 快照系统（adk-session SQLite）
infra-sandbox = []       # 沙箱执行环境（Phase 2 placeholder）
infra-rtk = []           # rtk 命令输出压缩集成

# ── interface/ layer ──────────────────────────
ui-tui = []              # ratatui TUI 交互模式
ui-review = []           # 交互式 Diff 评审（CLI + web）

# ── development only ──────────────────────────
dev-vt100 = []           # VT100 终端模拟测试
dev-e2e = []             # PTY E2E 测试
```

始终编译的功能（无 feature flag，config 层控制）：

| 功能 | Config 控制键 | 说明 |
|------|-------------|------|
| 7 个内置工具 | `tools.allowlist` / `tools.blocklist` | agent 核心能力，无工具不可用 |
| Hook 事件系统 | `hooks: []` | 空列表即 no-op，零开销 |
| 安全策略引擎 | `security.enabled` | 默认 deny-all，`globset` + `regex` |
| 重复检测 | `repeat_detection.enabled` | HashSet + 滑动窗口，防止死循环 |
| Print 模式 | CLI `--mode print` | 默认交互模式，仅 `owo-colors` |
| ACP 模式 | CLI `--mode acp` | IDE 集成，agent-client-protocol SDK |

## Capabilities

- `workspace-structure`: 单 crate 领域分层骨架 + feature flags 体系

## Impact

- 破坏性变更：`src/lib.rs` 重写，`src/main.rs` 重写
- 所有后续 change 在此骨架上填充各模块
- 不引入任何外部依赖（纯骨架 + 占位模块）
