# Debug 日志默认策略 — 探查结论

> 输出依据：`_tmp_prompts/04-logging-debug-default.md` 任务。供 c460 升格时参考。

## 现状

`src/app/cli/logging.rs::init_logging` 完全 env-driven：

1. `RUST_LOG` 已设置 → 用 `EnvFilter::try_from_default_env()`
2. `XYLITOL_DEBUG=1` 已设置 → 默认 filter `"xylitol=debug,warn"`
3. 否则 → 返回 `None`，所有 `tracing::` 宏为 no-op

**写入位置**：`<agent_dir>/logs/xylitol.log`（`~/.xylitol/logs/xylitol.log`），`0o600`，纯文件，不写 stdout/stderr。

**调用时机**：`app::cli::run` 最开头，早于任何 mode 分发。

## 建议改动

`init_logging` 末尾（return None 之前）加 `cfg(debug_assertions)` 兜底：

```rust
#[cfg(debug_assertions)]
{
    // debug 构建：默认启用文件日志，无需环境变量
    let filter = EnvFilter::new("xylitol=debug,warn");
    if let Some(file) = open_append(&log_dir, &log_path) {
        let _ = tracing_subscriber::fmt()
            .with_writer(file)
            .with_ansi(false)
            .with_env_filter(filter)
            .try_init();
        tracing::info!(target: "xylitol::logging", path = %log_path.display(), "debug-build logging enabled by default");
        return Some(());
    }
}
return None;
```

**效果**：
| 构建 | 无 env var | `XYLITOL_DEBUG=1` | `RUST_LOG=…` |
|---|---|---|---|
| debug | ✅ 自动写日志 | ✅ 同左（优先 env） | ✅ 同左（最高优先） |
| release | ❌ 不写 | ✅ 写 | ✅ 写 |

## 与测试/BDD 兼容性

- ✅ 无测试调用 `init_logging`（`rg` 确认）
- ✅ 无测试安装全局 subscriber（`rg` 确认）
- ✅ `try_init` 若全局 subscriber 已存在则静默 no-op
- ✅ 纯文件写，不干扰 stdout/stderr → TUI/print 均安全

## 实施位置

- 文件：`src/app/cli/logging.rs`
- 函数：`init_logging`
- 插入点：`else { return None; }` → 改为上述 `cfg(debug_assertions)` 分支后再 return None

## 不在此方案内

- `#[cfg(test)]` 下的 logging 行为（测试不应依赖文件日志）
- release 构建下额外开关（用户已可用 `RUST_LOG` / `XYLITOL_DEBUG`）
- 日志轮转/大小限制（初版不需要；用户 `tail -f` 后自行 truncate）
