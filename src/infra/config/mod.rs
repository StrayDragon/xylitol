pub(crate) mod error;
pub mod loader;
pub(crate) mod paths;
pub(crate) mod secret_env;
pub(crate) mod template;
pub mod types;
pub(crate) mod validate;

// ConfigValueResolver（r1821-1823）：spec 要求提供的配置值解析能力
// （literal / $VAR / ${VAR:-default} / `!command` 带预算执行与进程生命周期缓存）。
// 产品接入（r1912）：`load_from_paths` 装配 models.models 条目值时经
// `resolver::resolve_from_env` 展开表达式（见 loader.rs）。
pub mod resolver;
