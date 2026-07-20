# Design: c1400 移除 config.local（C）

## 目标心智

```text
项目协作：  .xylitol/config.yaml          （git）
本机密钥：  .xylitol/secret.env           （gitignore）
            + 可选 ~/.config/xylitol/secret.env
全局偏好：  ~/.config/xylitol/config.yaml （可选）
数据目录：  ~/.xylitol/{sessions,tokenizers,logs,…}
```

**无** `config.local.yaml`。

## 加载顺序（变更后）

1. global `config.yaml`
2. project `config.yaml`
3. `--config`

`secret.env`：global ← project overlay；OS env 永不被覆盖。

## 残留文件

| 情况 | 行为 |
|---|---|
| 存在 `config.local.yaml`/`.yml` | **不合并** |
| doctor | SHOULD 提示「已忽略，请改用 config.yaml + secret.env」（可选但推荐，避免静默迷惑） |
| 自动迁内容 | **禁止** |

## 实现触点

- `loader.rs`：删 global/project local 分支
- `migrate.rs`：停止复制 local 文件到 XDG（避免制造无效文件）
- 文案：architecture、example、secret.env.example、TUI 缺模型、`.xylitol/config.yaml` 注释
- 测试：`tests/tui_e2e/pty.rs` 等改写 `config.yaml`；loader 单测「仅有 local 时等于无该层」

## 风险

依赖 `config.local.yaml` 的本机配置在升级后**静默失效**（密钥若只在 local 会丢模型/鉴权）——接受；用户自行把内容搬进 `config.yaml` / `secret.env`。
