# Tasks

> 先加后删：新容器与注册表先与旧 `AppConfig` 并存，迁移调用点后再删旧字段。

- [x] 1.1 在 config 插件加入 YAML `Value` 层 merge key 展开与单测（覆盖 mapping / mapping 列表 / 父键优先 / 非法值报错）。[seam: config crate 单测]
- [x] 1.2 加入 `xylitol:` 引导段（`schema_version` / `unknown_keys` / `plugins`）；未知键两阶段校验（pre-boot 收集、boot 后判定，默认硬失败、`warn` 放宽），带单测。[seam: config crate 单测]
- [x] 2.1 kernel 新增 `ConfigSection` 静态清单与聚合 schema 渲染；config 插件改为从注册表取 section schema。[seam: kernel 单测 + schema 产物字节一致]
- [x] 2.2 `AppConfig` 改为 `xylitol` + `BTreeMap<String, Value>` 容器；插件 section 由插件自行反序列化；旧字段读取点逐个迁移。[seam: kernel 单测]
- [x] 3.1 外进程插件允许顶层 section；manifest 声明 `config_key` + `config_schema`，boot 后按 rutis 依赖顺序读取并校验。[seam: infra-plugin BDD]
- [x] 4.1 CLI 新增 `config schema|check|show|init` 四子命令，`check` 输出未知键与迁移提示。[seam: cli-entry BDD]
- [x] 4.2 删除 `configs/example.yaml` 与生成器；`configs/config.schema.json` 由注册表再生成；`checked_in_schema` 单测守住漂移。[seam: config crate 单测]
- [x] 4.3 移除 `include` 解析与相关单测；旧 `include` 键硬失败并给迁移提示。[seam: config crate 单测]
- [x] 5.1 更新 `runtime-config` / `cli-entry` / `infra-plugin` / `user-experience` specs 并跑 `llman-sdd validate` 与 BDD。
