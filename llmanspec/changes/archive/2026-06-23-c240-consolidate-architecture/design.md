# Design: c240-consolidate-architecture

## 1. Pi 文档引用清理

### 模式
所有 `//! Aligns with pi's ...` 行替换为模块实际职责描述。

**替换规则**：
```
//! Aligns with pi's {ts-module}. {original-description}
```
→
```
//! {中文或英文模块职责描述，独立且精准}
```

对于简单模块（如 `output_guard.rs`），保持英文描述但移除 pi 引用。

### 受影响文件（30 个）

core/ (0 — 已无 pi 引用)
agent/:
  - session.rs, loop.rs, prompt.rs, queue.rs, templates.rs, output_guard.rs,
    config_value.rs, bash_executor.rs, commands.rs, retry.rs, http_dispatcher.rs,
    tools/mutation.rs, tools/accumulator.rs, model/registry.rs, model/resolver.rs,
    trust/store.rs, trust/resolve.rs, auth/guidance.rs, provider/attribution.rs
infra/:
  - timing.rs, source_info.rs, resource/mod.rs, resource/loader.rs,
    session/mod.rs, session/cwd.rs, settings/storage.rs, settings/manager.rs
interface/:
  - rpc.rs

## 2. 微系统合并

### ToolManager 内联回 session.rs
- ToolManager（50 行, 4 方法）仅做 field 存储 + getter 委托
- 直接移除独立结构，`active_tools: Vec<String>` 和 `registry: ToolRegistry` 作为 AgentSession 的 field
- 方法直接实现为 session 方法

### ModelManager 简化
- ModelManager（119 行, 12 方法）保持独立
- 移除 `build_current_model()`（只做 `registry` 查找，由 session 的调用方直接构建）
- 移除 `set_thinking_level()` 的隐式持久化回调 → 由 session 的 `set_thinking_level()` 显式调用

## 3. 宏系统评估

产出 `docs/architecture/macro-registration.md`，评估以下候选：

| 候选 | 优点 | 缺点 | ROI |
|---|---|---|---|
| `#[tool]` 注册 | 编译期确定，启动快 | 无法动态加载；复杂参数难表达 | 中 |
| `#[command]` 静态表 | 零运行时开销 | 与扩展命令（skill）冲突 | 低 |
| `#[provider]` 匹配 | 减少 `build()` 匹配分支 | 需要过程宏 crate | 中-高 |

只做文档评估，实际宏生产代码留到后续 change。
