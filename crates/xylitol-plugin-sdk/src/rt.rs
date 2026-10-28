use std::future::Future;

use rutis::events::Mode;

pub use rutis::events::Call;
pub use rutis::host::{CtxHost, ErrorKind, FiberState, FiberView, TypeKey};
pub use rutis::{Ctx, Deps, Disposer, Error, Event, Plugin, Result};

// ~keep
// TODO(#sdk-version-init): SDK_API_VERSION 与 RUTIS_LOCK 目前只是编译期常量，
// 无运行期消费者（host.describe 不上报）。首个正式版（当前 version 为 0.0.0-pre）
// 发布前须决定：是否让它成为真正的插件兼容协商信号，或整体移除。
pub const SDK_API_VERSION: &str = "0.6.0";

pub const RUTIS_LOCK: &str = "0.17.0";

pub trait CallExt<E: Event> {
    fn chain(self) -> impl Future<Output = Result<Option<E::Value>>> + Send;
}

impl<E: Event> CallExt<E> for Call<E> {
    async fn chain(self) -> Result<Option<E::Value>> {
        if self.mode() == Mode::Waterfall {
            self.next().await
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use rutis::host::{CtxHost, ErrorSink};

    use super::*;

    #[derive(serde::Serialize, serde::Deserialize)]
    struct ChainEv {
        n: u32,
    }

    impl Event for ChainEv {
        const NAME: &'static str = "sdk/call-chain";
        type Value = u32;
    }

    async fn wait_done(done: &AtomicUsize) {
        for _ in 0..32 {
            if done.load(Ordering::SeqCst) >= 1 {
                return;
            }
            tokio::task::yield_now().await;
        }
        panic!("listener did not finish");
    }

    #[tokio::test]
    async fn chain_on_emit_does_not_fail() {
        let errors = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let sink_errors = errors.clone();
        let sink: ErrorSink = Arc::new(move |e| {
            sink_errors.lock().expect("sink").push(e.to_string());
        });
        let ctx = Ctx::root_with_sink(tokio::runtime::Handle::current(), sink);
        let hits = Arc::new(AtomicUsize::new(0));
        let done = Arc::new(AtomicUsize::new(0));
        ctx.on::<ChainEv>({
            let hits = hits.clone();
            let done = done.clone();
            move |_, call| {
                let hits = hits.clone();
                let done = done.clone();
                async move {
                    hits.fetch_add(1, Ordering::SeqCst);
                    let rest = call.chain().await?;
                    done.fetch_add(1, Ordering::SeqCst);
                    Ok(Some(rest.unwrap_or(4)))
                }
            }
        })
        .expect("on");
        ctx.emit(ChainEv { n: 0 }).expect("emit");
        wait_done(&done).await;
        assert_eq!(hits.load(Ordering::SeqCst), 1);
        assert!(
            errors.lock().expect("sink").is_empty(),
            "emit + chain must not hit the error sink"
        );
        let _ = ctx.shutdown().await;
    }

    #[tokio::test]
    async fn chain_on_waterfall_equals_next() {
        let ctx = Ctx::root().expect("root");
        let hits = Arc::new(AtomicUsize::new(0));
        ctx.on::<ChainEv>({
            let hits = hits.clone();
            move |_, call| {
                let hits = hits.clone();
                async move {
                    hits.fetch_add(1, Ordering::SeqCst);
                    let rest = call.chain().await?;
                    Ok(Some(rest.unwrap_or(9)))
                }
            }
        })
        .expect("on");
        let got = ctx.waterfall(ChainEv { n: 1 }).await.expect("waterfall");
        assert_eq!(got, Some(9));
        assert_eq!(hits.load(Ordering::SeqCst), 1);
        let _ = ctx.shutdown().await;
    }
}
