# 派工 prompt：余下 cap 的 valid_scope / src 路径钉（c2210 第二刀）

把下面整段交给执行 agent。摩擦 1–10 已合入 main（`76497cdb`）。本切片只扫 **尚未动过的 capability** 里残留的路径钉。

---

## 难度

中。机械面是放宽 `valid_scope`；危险面是把 BDD **夹具路径**当成组织钉删掉。吃不准就 `keep` 并在 commit 里标出。

## 在哪开工（硬）

**必须独立分支，禁止在默认分支改 `llmanspec/specs/**`。** 主仓正在改 PreviewInject：

```bash
wt switch --create --no-cd -y c2210-spec-remainder
cd ../xylitol.c2210-spec-remainder
eval "$(just cargo-wt-env)"
```

不要在当前 `main` 工作树上直接改 specs。

## 读（按序）

1. `llmanspec/AGENTS.md` → 「spec 约束层级」
2. `llmanspec/changes/c2210-update-specs-product-level/research/spec-rigidity-and-product-level.md`
3. 已落地对照：`git show fbddfe86 --stat`（第一刀改了哪些 cap，**不要重做**）

尺子：**实现能改、对外行为不变 → 不进 spec。** 夹具文件名（场景里「假定存在文件 `src/hello.rs`」）是测试数据，不是代码组织钉。

## 做

1. `rg -n 'src/' llmanspec/specs --glob '*.toon' --glob '*.feature'`，按 cap 分拣：

| 形态 | 动作 |
|---|---|
| `valid_scope` 钉到具体 `src/...` 文件/目录 | 放宽为层/包（infra 层、agent 层、产品 TUI 面、xylitol-tui 包、xylitol-ai-bridge 包…） |
| requirement statement 钉模块路径 / 类型名 / 行号 | `rewrite-product`：留 WHAT，去路径；或 `move-to-AGENTS` 一句指针（禁止把长教程搬进 AGENTS） |
| feature「打开/检查具体源文件」当 Given | 改行为断言，或删该场景 |
| 防复活（「`src/foo` 不得再存在」） | `delete`（先 `rg` 确认对象不存在） |
| BDD 夹具路径（`假定 存在文件 "src/hello.rs"`、工具参数 path） | **keep** |
| 第一刀已改过的 cap | **不要再改**（layer-architecture / app-tui-host / chrome / playground / qa-gate 等） |

2. 已知余量（2026-08-14 扫描，执行时以当时 `rg` 为准）：

- toon `valid_scope`：`infra-diagnostics` / `infra-otel` / `infra-provider` / `infra-git` / `infra-clipboard` / `infra-bash` / `infra-mcp` / `infra-image` / `infra-process` / `infra-network` / `runtime-resource-discovery` / `runtime-model-registry` / `agent-prompt` / `agent-hooks` / `agent-todo` / `server-core` / `cli-print` / `test-standards` / `test-fake-provider` / `test-provider-integration` / `test-bdd` / `package-ai-bridge*` / `package-tui-markdown` / `package-tui-keybindings`
- feature 组织钉（**不是**夹具）：`package-ai-bridge.feature`「审查 src/infra/provider」；`server-core/server-runtime.feature`「src/app/server 存在且 src/server 不再存在」；`test-provider-integration.feature`「检查 src/agent/provider 是否存在」；`app-tui.feature`「检查 src/app/tui 导入」；`test-bdd` tb4 钉 `src/infra/config`
- `agent-tools.feature` 里大量 `src/*.rs` = **夹具，keep**
- `domain-security.feature` `"/project/src/main.rs"` = 沙箱路径，keep

3. 不要一次重写这些 cap 的全部 req；只动路径钉 / 防复活 / 「打开源文件」场景。

## 禁止

- 在默认分支改 `llmanspec/specs/**`
- 改产品 Rust（除非 AGENTS 指针必须动的一句）
- 删 playground / DESIGN.md
- 动 c2230 / PreviewInject / 死码拿不准项
- 把 `agent-tools.feature` 夹具改成「不写 src/」

## 验证

- `llman sdd validate --specs`（或至少你改过的 cap）全绿
- 动过的 cap 的 `valid_scope` 不再钉到具体 `.rs` 文件
- feature 场景不再以「打开/检查具体源文件」为 Given（夹具路径除外）

## 提交

分支 `c2210-spec-remainder`。标题：`docs(spec): loosen remaining valid_scope path nails`。

Commit 按 cap 列 keep/rewrite/delete。不要 ff 进 main。
