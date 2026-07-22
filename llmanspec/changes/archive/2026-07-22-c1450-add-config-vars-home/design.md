# Design: c1450-add-config-vars-home

## Decision

在既有 minijinja 配置模板上下文中新增命名空间 `vars`，**仅**注入键 `home`。

| 键 | 来源 | 失败行为 |
|---|---|---|
| `vars.home` | `dirs::home_dir()` → 绝对路径字符串 | `None` → strict 模板错误（加载失败） |
| `vars.<其它>` | 不注入 | Strict undefined（与缺 `secret.KEY` 同类） |

不新增第二套插值语法；不改 `env` / `secret`。

## Why only `home`

- 共享 `config.yaml` 最常见外漏是用户 home 绝对路径（MCP stdio `command`）。
- `project` / `cwd` 会鼓励把机器/会话相关路径写进可分享配置，扩大信息面；本 change **刻意不做**。
- `{{ env.HOME }}` 已可用，但：
  - 依赖进程环境是否设置 `HOME`；
  - 产品叙事上希望可分享配置用稳定 `vars.home`，而不是散落 `env`。

## Implementation seam

- 唯一改动点：`src/infra/config/template.rs` 的 `render_config_template` context。
- Loader 无需改签名：home 解析在模板层完成。
- 单测：成功渲染；未知 `vars.*` 失败。不扩 BDD step（与 rc22 同策略）。

## Alternatives rejected

| 方案 | 为何不做 |
|---|---|
| 只文档化 `{{ env.HOME }}` | 仍依赖 env；无产品级「仅 home」边界 |
| 支持 `vars.project` 等同批上线 | 超出用户范围与安全意图 |
| 预展开 `~` 路径语法 | 新语法面；与现有 mustache 不一致 |
