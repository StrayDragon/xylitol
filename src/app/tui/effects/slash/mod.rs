//! PendingSlash side-effect arms (c1170). Still drained only via drain_pending.

mod compact;
mod debug;
mod misc;
mod model;
mod session;

use xylitol_tui::Terminal;

use crate::app::core::driver::XyDriver;
use crate::app::tui::commands::PendingSlash;
use crate::app::tui::host::HostSession;

pub(super) async fn handle_slash<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
    slash: PendingSlash,
) {
    match slash {
        PendingSlash::Exit => misc::exit(session).await,
        PendingSlash::OpenModels => model::open(session, driver).await,
        PendingSlash::SetModel(model_id) => model::set(session, driver, model_id).await,
        PendingSlash::DebugScene(scene) => debug::run(session, driver, scene).await,
        PendingSlash::OpenTree => session::open_tree(session).await,
        PendingSlash::ForkAtLeaf => session::fork_at_leaf(session, driver).await,
        PendingSlash::Compact { instructions } => {
            compact::run(session, driver, instructions).await;
        }
        PendingSlash::Export { path } => session::export(session, driver, path).await,
        PendingSlash::Import { path } => session::import(session, path).await,
        PendingSlash::SessionDump => session::dump(session, driver).await,
        PendingSlash::OpenSessionResume => session::open_resume(session, driver).await,
        PendingSlash::SessionNew => session::new_session(session, driver).await,
        PendingSlash::SessionClone => session::clone_session(session, driver).await,
        PendingSlash::SessionName { name } => session::name(session, driver, name).await,
        PendingSlash::Reload => misc::reload(session).await,
        PendingSlash::Trust { mode } => misc::trust(session, driver, mode).await,
        PendingSlash::HistoryCopyLast => misc::history_copy_last(session, driver).await,
        PendingSlash::Theme { arg } => misc::theme(session, arg).await,
        PendingSlash::OpenMcp => misc::open_mcp(session, driver).await,
        PendingSlash::Usage(msg) => misc::usage(session, msg).await,
    }
}
