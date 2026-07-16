# Design — c1100-update-runtime-context-hot-reload

## Decision: `XyReloadable`（窄端口，非插件表）

Rust 适合用 **trait 约束**统一「可热重载」语义，但本仓 **不是** 扩展市场：

| 做 | 不做 |
|---|---|
| `XyReloadable { type Outcome; fn reload(&mut self) -> Outcome }` | `Vec<Box<dyn XyReloadable>>` 注册表 |
| 关联类型保留各域诊断形状 | 强行统一成一个全局 `ReloadError` |
| `DefaultResourceLoader` / 未来 settings 等实现 | 强迫 app 层 keybindings 也实现该 port（c1090 已是 app-local） |

`/reload`（c1120）将是 **显式顺序编排**（keybindings → context → themes → …），与 pi 一致，而不是遍历 trait object。

## Resource loader

```text
reload():
  clear cached vectors + diagnostics
  load_all()   // existing: context, prompts, skills, themes, SYSTEM/APPEND
```

`XyResourceLoader` **不**强制继承 `XyReloadable`（避免破坏仅只读 mock）；具体 `DefaultResourceLoader` 同时实现两者。

## Agent / Driver

新增（命名以实现为准）：

```rust
AgentCapabilities::apply_prompt_resources(
    context_files: Vec<(String, String)>,
    system_prompt: Option<String>,
    append_system_prompt: Vec<String>,
)
// → 写 prompt_opts → rebuild_system_prompt()
// MUST NOT: append/replace session history / store entries
```

`AgentRuntime` / `InProcessDriver` 转发。语义对齐 `ar6`：只影响**下一轮** `run`。

## app/core 助手（Trust）

与 bootstrap 同构：

- `project_trusted == false` → loader cwd = temp（跳过项目 `.xylitol/` 与向上 walk 的项目 AGENTS）
- `system_prompt = loader.get_system_prompt().or(config_system_prompt)`
- 返回轻量 report（context 条数、是否有 system、diagnostics 可选）

TUI host / c1120 只调此助手，不 import `infra::resource`。

## Trade-offs

- 不在本 change 持有长寿命 `ResourceLoader` 于 HostSession：每次 reload `new` 或 `reload` 均可；trait 为后续持有者与 CLI doctor 复用铺路。
- config profile `system_prompt` 需调用方传入 fallback（与 bootstrap `or` 一致）；host 若无配置可传 `None`（仅磁盘）。
