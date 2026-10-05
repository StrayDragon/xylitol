pub(crate) mod error;
pub(crate) mod loader;
pub(crate) mod paths;
pub(crate) mod secret_env;
pub(crate) mod template;
pub mod types;
pub(crate) mod validate;

// ConfigValueResolver（r1821-1823）：spec 要求提供的配置值解析能力
// （literal / $VAR / ${VAR:-default} / `!command` 带预算执行与进程生命周期缓存）。
// 行为由 BDD 真步骤证据覆盖；产品接入点在 r1824 provider 注册配置解析——届时
// resolver 将成为 crate 公共 API，remove 本 allow。
#[allow(dead_code)]
pub mod resolver;
