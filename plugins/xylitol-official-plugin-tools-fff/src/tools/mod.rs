pub mod find;
pub mod grep;
pub mod index;
pub mod timeout;

pub use find::FffFindTool;
pub use grep::FffGrepTool;
pub use index::IndexCache;

use std::sync::Arc;
use std::time::Duration;

use xylitol_plugin_sdk::XyTool;

pub fn fff_tools() -> Vec<Arc<dyn XyTool>> {
    let cache = IndexCache::new(true, Duration::from_secs(10));
    vec![
        Arc::new(FffGrepTool::new(cache.clone())),
        Arc::new(FffFindTool::new(cache)),
    ]
}
