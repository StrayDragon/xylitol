# Design: c1960 tool_search + MCP Deferred 发现

> **轨 B**（非开箱）。轨 A 定稿冻表见已归档 [`c1900`](../archive/2026-08-05-c1900-update-mcp-first-turn-tool-freeze/design.md)。
> **调研**：[`c1900 research`](../archive/2026-08-05-c1900-update-mcp-first-turn-tool-freeze/research/responses-tools-stable-id-and-resume-mcp-2026.md) + [`docs/research/responses-context-layout-and-cache-2026.md`](../../../docs/research/responses-context-layout-and-cache-2026.md) §4.2。
> **现状钩子**：`ContextPolicy.tools_mode` 已有 `Search`；`ToolsMode::Search` ⇒ `allows_midturn_tools_rewrite=false`；开箱默认仍 `Full`。

## 目标

MCP 工具极多或不愿首轮灌全表时：顶栏稳定为 **核心 + 元工具**；MCP 进内部 registry（Deferred）；发现经 `tool_search`（hosted / client-function 双轨）尾部加载。靠 **声明分流**，禁止运行时猜端能力。

## 决策表（Designed 钉死）

| ID | 议题 | 选择 |
|---|---|---|
| Q1 | wire 双轨 | **Hosted**（`type: tool_search` + `defer_loading` / namespace·MCP 声明）与 **ClientFunction**（`function` 名 `tool_search` + 约定 output / 方言 upsert）并存；按声明选一，不混用假 hosted |
| Q2 | 能力探测 | **仅手动声明**；禁止 HTTP 探测 / 静默试发决定轨 |
| Q-WP | WirePolicy 声明分流 | 见下节 **钉死** |
| Q3 | 检索后端 | **A+B**：默认 **A=进程内内存 BM25**；B=显式 sidecar 检索（可选） |
| Q4 | sidecar | **仅显式开启**才用；默认不另开 model 请求脏主轨迹 |
| Q5 | 开箱 Search | **作废作默认**：开箱 = 轨 A（`tools_mode=Full` + c1900 门闸冻表）；Search 为声明后可选轨 |
| Q6 | description 刷新 | 元工具 / deferred 描述变更 ⇒ **知悉 cache bust**；刷新源 = registry 当前快照（不另造第二描述 SSOT） |
| Q7 | forge 形状 | Assembler/adapter **严格按声明**出线：Hosted→原生 item；ClientFunction→function 形；Unsupported→不得假装 Search 出线 |
| Q8 | 方言被迫改表 | **按 name upsert**（禁同名双行）；知悉必 cache miss；不得盲 append 同名 |
| Q9–Q11 | 与 c1900 | 继承：工具宜稳；去掉 pending 热并；Full 轨仍首条门闸定稿。Search 轨门闸 = registry settle 后 **冻顶栏（core+meta）**，MCP 不进顶栏 |
| Q14/Q16 | reload/resume × Search | idle `/reload`：重建 registry 索引 + **保持顶栏稳定**（不把 MCP 灌回顶栏）；历史已 `tool_search_output` 按官方 append-only 语义；不可执行则确定性 unavailable output，**禁止**静默按名重绑异语义。指纹续冻（Full）另波仍适用 Full；Search 不靠「全表指纹」冒充 remap |
| Q17 | 双轨产品 | 两条线都要能落地；验证 Hosted/Client **不得**用 Ornith 冒充 |
| Q-V | 活测 provider | **Start 前**钉假 provider 矩阵即可 apply；真网关清单 apply/verify 前另钉（OpenAI 原生优先；Ornith 只证 ClientFunction/剥离） |

## WirePolicy 声明分流（Q-WP · 钉死）

### 问题

- 口语「靠 WirePolicy 声明」与 live `pab20` 冲突：`ExtraPolicy` **禁止**塞 `tool_search` / `defer_loading` 等 agent 能力。
- 同时 Assembler/adapter **必须**知道出线形状（hosted item vs function 形），否则方言会静默剥离或 400。

### 分层

```text
ContextPolicy.tools_mode          ← agent：是否走 Search 轨（默认 Full）
WirePolicy.tool_search_wire       ← wire：出线形状声明（非 ExtraPolicy）
内部 ToolRegistry / BM25          ← 发现数据面（不进顶栏）
```

| 层 | 职责 | MUST NOT |
|---|---|---|
| `ToolsMode` | Search on/off；Search⇒禁 mid-turn 改顶栏 | 独自决定 hosted vs function |
| `WirePolicy.tool_search_wire`（新字段名可微调，语义固定） | `Unsupported` \| `Hosted` \| `ClientFunction` | 进 `ExtraPolicy`；运行时探测改写 |
| `ExtraPolicy` | 仅既有 cache / `previous_response_id` 位 | 任何 tool_search / defer 位 |

### 生效规则

1. **开箱**：`tools_mode=Full` ∧ 任意 `tool_search_wire` → 行为 ≡ c1900（忽略 Search 出线）。
2. **启用 Search**：仅当 `tools_mode=Search` **且** `tool_search_wire ≠ Unsupported`。
3. `tools_mode=Search` ∧ `Unsupported` → **硬失败或显式回退 Full**（实现选可观测拒绝优先；禁止静默发 hosted item）。
4. 声明真源：**code-first**（`defaults.rs` / 命名 `compat` 轮廓映射）；本波 **不**新扩 YAML 自由旋钮。测试可用结构体字面量覆盖。
5. 默认板意向：`Compat::Generic` / `Deepseek` → `ClientFunction` 或 `Unsupported`（Ornith lab：`Hosted` 禁止）；OpenAI 第一语言轮廓可声明 `Hosted` 或 `ClientFunction`。**具体默认常量在 apply 任务钉死并单测锁住。**

### 与 c1880/c1940 关系

- 不破坏「`extra_policy` 仅 req/resp 布尔」。
- `tool_search_wire` 是 **wire 出线轮廓**，不是 ContextPolicy 旋钮；agent 不得绕过它自行 forge hosted。

## 架构

```text
MCP settle ──► 内部 Registry（core ∪ armed MCP）──► BM25 索引（内存）
                      │
        tools_mode=Full (轨 A)          tools_mode=Search ∧ wire≠Unsupported (轨 B)
                      │                              │
              门闸 → 冻全表                    门闸 → 冻顶栏(core+tool_search[+hosted 声明])
              provider tools[]=FROZEN          MCP Deferred；发现不改顶栏
                      │                              │
                      └──────── Assembler(WirePolicy) ──────► /v1/responses
```

### 轨 B 请求环

1. 顶栏：`core` + 元工具（按 `tool_search_wire` forge）[+ Hosted 时 deferred 声明子集]。
2. 模型调用 search → client：handler 查 BM25 → 回传 `tool_search_output`（或方言约定的 function output + upsert）。
3. Hosted：服务端产出 call/output；客户端不伪造。
4. 已加载集 **append-only**；禁用/改写 loaded set ⇒ 从该点 bust cache（官方语义；产品接受）。

### 门闸复用

- 仍复用 c1900「首条可提交、generate 等 settle/超时」心智与超时常量。
- 差分：Search 定稿对象 = **顶栏稳定集 + registry 索引世代**，不是「全 MCP 进 `tools[]`」。
- settle 后热并 **只**进 registry/索引，**禁止**扩 Search 顶栏。

## 非目标

- 把 Search 设为开箱默认
- 用 Ornith/llama.cpp 冒充 Hosted 通过
- Anthropic Tool Search 方言
- `previous_response_id` 当 tool identity
- 稳定 definition id / placeholder remap（调研已否）
- 索引落盘；自动 sidecar
- 本波扩 YAML `tools_mode` / `hosted_tool_search` 用户面（code-first）

## 测试 seam（Start 后 Specs landing 用）

| Seam | 覆盖 |
|---|---|
| `WirePolicy` 字面量 × Assembler | Hosted / ClientFunction / Unsupported 出线 diff；ExtraPolicy 无 search 位 |
| Agent registry + BM25 | Deferred 不进顶栏；search 命中返回定义；索引随 reload 重建 |
| Fake provider（声明 Hosted/Client） | 轨迹尾部加载；顶栏跨轮名表稳定 |
| 方言 ClientFunction | function 形可调用；upsert 按名；禁同名双行 |
| 回归轨 A | 默认 `Full` 行为不变（c1900 门闸/冻表） |

**禁止**以 Ornith live 作为 Hosted MUST 证据。

## Start readiness

| 项 | 状态 |
|---|---|
| `depends_on` c1880/c1890/c1900 | 已归档 |
| proposal / design / tasks | 本波补齐 → **Designed** |
| Open Questions | 上表已钉；真网关清单留 apply 前 |
| Branch binding / Specs landing | **未做**（本波硬禁止 `change start`） |
| 代码 | **未做** |

**下一步（人工触发）**：干净树默认分支 → `llman-sdd change start c1960-add-tool-search-mcp-discovery` → Specs landing（`package-ai-bridge` / `agent-runtime` / `infra-mcp` 等）→ `readyToImplement` → apply。
