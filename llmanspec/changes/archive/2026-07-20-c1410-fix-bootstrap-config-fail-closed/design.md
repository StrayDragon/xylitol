# Design: c1410 bootstrap 配置 fail-closed

## 问题链（已复现）

```text
.xylitol/config.yaml 注释含 {{ secret.KEY }}
  → minijinja undefined value
  → ConfigLoadFailed (warning only)
  → registry 空 → OPENAI_API_KEY 注册 id=gpt-4o
  → TUI 进入，footer 显示 gpt-4o
```

用户期望配置为 `qwen`，实际看到的是无关默认名。

## 目标行为

```text
load_app_config Err
  → BootstrapError::ConfigLoadFailed(msg)   # 硬失败，全表面
  → CLI eprintln Error: … ; 非零退出
  → 不进入 TUI / 不 print 一轮

load Ok 且 models 可注册数 == 0（配置面存在意图但空）
  → BootstrapError（ce2）硬失败，指向配置
  → MUST NOT env 填 gpt-4o

load Ok 空配置（无文件 / 默认空）+ 仅有 API key
  → MAY 把 key 可知的 provider 放进「可发现」列表（若保留）
  → MUST NOT 自动 select_model(gpt-4o)
  → current_model == None；展示 NOT-SET
  → TUI preflight NoModelSelected 拦进面
  → 用户须 --model 或写 config.yaml
```

## 与 m3 的边界

| 能力 | 行为 |
|---|---|
| `default_model_id_for_provider("openai")` | 仍返回真实 ID（如 `gpt-4o`）— **解析辅助** |
| bootstrap 空 registry 自动 register+select | **禁止**冒充「已配置默认」；未显式选择 → `NOT-SET` / 未选中 |

## 实现触点

1. `BootstrapError` 增 `ConfigLoadFailed(String)`（或复用并提升现 Warning）
2. `resolve_assembly`：load Err → `return Err(...)`；有配置零模型 → Err；删/改 env 自动 select
3. `cli/mod.rs`：`render_warnings` 去掉 ConfigLoadFailed 的 fallback 文案；match 新 Error
4. TUI `HostSession` / footer 未选中文案 → `NOT-SET`
5. 仓内 `.xylitol/config.yaml`：注释改为不含未转义 `{{ ... }}`（如写成 `` `{{` secret.KEY `}}` `` 或「secret.KEY 占位」）

## 测试策略

- 单元：`resolve_assembly` / bootstrap — 坏模板 YAML → Err；有配置零模型 → Err；仅 API key → 无 current 选中
- TUI/CLI 文案：未选中为 `NOT-SET`
- 不强制新 BDD step 若现有 unit 已覆盖；live feature 场景作合约例子，实现期接 harness 或文档场景按项目惯例
