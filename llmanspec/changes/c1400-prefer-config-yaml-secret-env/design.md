# Design notes (purpose-draft): c1400

## 目标心智

```text
项目协作：  .xylitol/config.yaml     （git）
本机密钥：  .xylitol/secret.env      （gitignore）+ 可选 ~/.config/xylitol/secret.env
全局偏好：  ~/.config/xylitol/config.yaml  （可选；少用）
数据目录：  ~/.xylitol/{sessions,tokenizers,logs,…}  （不是 AppConfig 主战场）
```

## 与今日 loader 的关系

保持深合并顺序直到选 C：

1. global `config.yaml`
2. global `config.local.yaml`  ← 文档降级
3. project `config.yaml`       ← 主推
4. project `config.local.yaml` ← 文档降级
5. `--config`

`secret.env`：global ← project overlay；OS env 永不被覆盖。

## 升 full 决策点

| 选项 | 行为 | 风险 |
|---|---|---|
| A 文档 only | 零代码 | 旧习惯仍在 |
| B + hint | 小改 | 噪声 |
| C 默认关 local | 需 env 打开 | 破坏依赖 local 的人 |

默认建议：**先 A+B，C 另 change**。
