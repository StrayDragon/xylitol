# Design：显式模型配置鉴权

## 决策

| 题 | 选择 |
|---|---|
| 注册门槛 | YAML 别名存在即 register；与 key 是否可解析无关 |
| `api_key` 省略 | 空串；**禁止** kind 级 env 回落 |
| `api_key: ""` | 同省略（已有：不回落） |
| `api_key: "{{ secret.X }}"` | 渲染后非空则用；渲染后空=空串 |
| 无 YAML 模型 | **删除** env→default_model_id 注入分支；空 registry → `NoModelsAvailable` / `ConfigLoadedZeroModels`（有 YAML 层但零条目） |
| Fake provider | 测试用 Fake 可继续空 key（既有） |

## 行为

```mermaid
flowchart TD
  YAML["YAML models.models 条目"] -->|每条| Reg["register 别名\napi_key=显式或空"]
  YAML -->|零条目且有 YAML 层| Fail1["ConfigLoadedZeroModels"]
  NoYAML["无 YAML 模型层"] --> Fail2["NoModelsAvailable\n禁止 env 造模"]
  Reg --> List["list-models / 选模 OK"]
  Reg --> Call["generate"]
  Call -->|key 空| Err["鉴权引导 m7/ux4"]
```

## 非目标

- `/login` OAuth 产品化
- 自动从 models.dev 填 key
- 改变 `compat`/`api` 解析
