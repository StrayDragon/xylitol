# design：provider 配置值 resolver 表达式（r1912）

## 决策表

| 决策点 | 选项 | 决定 | 理由 |
|---|---|---|---|
| 接入层 | serde 反序列化层 vs loader 后处理 vs 消费点 | **loader 后处理**（`load_from_paths` 尾部） | 唯一入口；不侵入 serde，绕过 loader 的直接 `from_str` 测试零影响；消费点（resolve_model / bootstrap 注册）自动获得解析值 |
| 触发条件 | 全字段一律解析 vs 仅 `$`/`!` 开头 | **仅 `$`/`!` 开头** | 零成本默认；字面 `$` 开头误伤极小且 unbound 报错可读（与 minijinja strict 同精神） |
| 失败语义 | 保留原文 vs 拒绝装配 | **拒绝装配（LoadError，alias:field 上下文）** | 与 minijinja strict（缺失 key 即错）一致；避免静默错误值进入请求头/key |
| lookup 源 | resolver 内置 env vs 注入 | **进程 env**（resolver 已支持 lookup 注入；产品路径用 `std::env::var`，secret.env 已注入进程环境） | 与 `{{ secret.X }}` 同一 env 来源；BDD 用唯一变量名 set_var 有先例 |
| minijinja 顺序 | 先渲染 vs 先 resolver | **先渲染，后 resolver** | 模板与 `$`/`!` 语法不重叠；渲染产出仍可被表达式引用 |
| 范围字段 | 全部 XyModelConfig 字段 | `api_key`/`model`/`base_url`/`api`/`compat` | 即 r1824 provider 注册可配置面；headers/tokenizer 明确不 scope |

## 风险与缓解

- **BDD 现网配置含字面 `$`/`!` 开头的 model 值**：本仓配置模型条目值均为普通字符串；
  若存在则 unbound 会报错——以单测 + qa 全量覆盖确认。
- **`!command` 在每次加载时执行**：resolver 已有进程生命周期缓存（r1823），装配不重复执行。
- **错误信息**：必须含 alias 与字段名，利于多模型配置定位。
