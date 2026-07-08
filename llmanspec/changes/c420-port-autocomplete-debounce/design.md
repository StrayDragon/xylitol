# Design — c420-port-autocomplete-debounce

## 决策 1：async trait 实现方式 — 单独方法 vs 修改签名

### 选项

| 方案 | 做法 | 结论 |
|---|---|---|
| A. trait 增加 `async_get_suggestions` 方法 | trait 保留同步 `get_suggestions`，增加独立 async 方法 | ✅ 采用 |
| **B. 修改现有 `get_suggestions` 签名** | 改为返回 `Option<Future<...>>` 或直接用 async_trait | ❌ 拒绝 |

### 理由

- **向后兼容**：现有同步调用者（当前 editor.rs 桩）不受影响。async 是新能力，不影响现有测试。
- **不使用 async_trait crate**：xylitol-tui 目前零 proc-macro 依赖，避免引入。async 方法返回 `Pin<Box<dyn Future>>`或更简单的模式。
- **更简单的方案**：`CombinedAutocompleteProvider` 增加一个 `async_get_suggestions` 方法（直接 `async fn`），`DebouncedAutocomplete` 直接调用它。不需要 trait 对象动态分发——`DebouncedAutocomplete<P>` 是编译期单态。
- **pi 对齐**：pi 的 `getSuggestions` 签名就是 async，但 pi 没有同步路径。xy 保留同步方法作为可选的后门。

### 后果

- `AutocompleteProvider` trait 保持纯同步（不变）
- `CombinedAutocompleteProvider` 增加 public `async get_suggestions_async()` 方法（不在 trait 里）
- `DebouncedAutocomplete<P>` 对泛型 P 不做 trait bound，直接调用 `get_suggestions_async()`

## 决策 2：fd 子进程 vs Rust ignore crate

### 选项

| 方案 | 做法 | 结论 |
|---|---|---|
| **A. spawn fd 子进程** | `std::process::Command::new("fd")`，对齐 pi 行为 | ✅ 采用 |
| B. 引入 `ignore` crate | Rust 库，不依赖系统 fd，但需新依赖 | ❌ 拒绝 |

### 理由

- **完全对齐 pi**：pi 用 `spawn(fdPath, args)`，参数完全一致
- **fd 是可选依赖**：`fd_path: Option<String>`，None 时自动回退到现有的 `read_dir` 非递归桩
- **不引入新依赖**：ignore crate 是外部依赖，预 1.0.0 期间尽量少引入
- **取消机制简单**：SIGKILL 子进程即可，不需要 tokio::spawn + select

### 后果

- `walk_directory_with_fd` 签名为: `fn walk_directory_with_fd(base_dir: &str, fd_path: &str, query: &str, max_results: usize, ct: CancellationToken) -> Vec<(String, bool)>`
- 返回 `Vec<(path: String, is_directory: bool)>`
- CancellationToken 取消时子进程被杀，返回空 vec

## 决策 3：DebouncedAutocomplete 设计 — 泛型 wrapper

### 选项

| 方案 | 做法 | 结论 |
|---|---|---|
| **A. 泛型 struct wrapper** | `DebouncedAutocomplete<P>` 持有 `P` + `delay: Duration` | ✅ 采用 |
| B. 独立函数 | `debounce_get_suggestions(provider, ...)` 函数 | ❌ 拒绝 |

### 理由

- **状态封装**：debounce 需要持有上一个 `CancellationToken`（用于 abort），wrapper 是最干净的封装
- **类型安全**：泛型约束编译期确定，无需 `Box<dyn AutocompleteProvider>`
- **pi 对齐概念**：pi 的 editor 持有 `this._autocompleteController`（和 debounce 逻辑），xbwrapper 封装同样概念

### 后果

```rust
pub struct DebouncedAutocomplete<P> {
    provider: P,
    delay: Duration,
    last_token: Option<CancellationToken>,
}

impl<P> DebouncedAutocomplete<P> {
    pub fn new(provider: P, delay: Duration) -> Self { ... }
    pub async fn get_suggestions(&mut self, ...) -> ... { ... }
}
```

## 非目标

- 不在 editor 中集成（c430 范围）
- 不修改 `AutocompleteProvider` trait（保留同步后门）
- 不引入 async_trait proc-macro（零 proc-macro 依赖）
