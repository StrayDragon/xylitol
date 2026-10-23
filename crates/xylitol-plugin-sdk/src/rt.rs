pub use rutis::{
    BoxFuture, Call, CordisError, Ctx, DisposeWaitError, Effect, ErrorSink, Event, EventBus,
    EventKey, EventOptions, EventPattern, FiberState, FiberView, Listener, Mode, Plugin,
    PluginFactory, PluginId, Snapshot, SyncCall, SyncListener, Terminal, TypeKey, Typed,
    TypedFactory, TypedPlugin, TypedPluginFactory,
};

pub const SDK_API_VERSION: &str = "0.2.0";

pub const RUTIS_LOCK: &str = "0.15.0";

pub trait CallExt<'a, V> {
    fn chain(self) -> BoxFuture<'a, Result<Option<V>, CordisError>>;
}

impl<'a, V> CallExt<'a, V> for Call<'a, V>
where
    V: serde::Serialize + serde::de::DeserializeOwned + Send + 'static,
{
    fn chain(self) -> BoxFuture<'a, Result<Option<V>, CordisError>> {
        if self.mode() == Mode::Waterfall {
            self.next()
        } else {
            Box::pin(async { Ok(None) })
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    #[derive(serde::Serialize, serde::Deserialize)]
    struct ChainEv {
        n: u32,
    }

    impl Event for ChainEv {
        const NAME: &'static str = "sdk/call-chain";
        type Value = u32;
    }

    struct ChainOnce {
        hits: Arc<AtomicUsize>,
        done: Arc<AtomicUsize>,
        value: u32,
    }

    impl Listener<ChainEv> for ChainOnce {
        fn call<'a>(
            &'a self,
            _: &'a Ctx,
            _: &'a ChainEv,
            call: Call<'a, u32>,
        ) -> BoxFuture<'a, Result<Option<u32>, CordisError>> {
            Box::pin(async move {
                self.hits.fetch_add(1, Ordering::SeqCst);
                let rest = call.chain().await?;
                self.done.fetch_add(1, Ordering::SeqCst);
                Ok(Some(rest.unwrap_or(self.value)))
            })
        }
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
        ctx.events()
            .on(
                &ctx,
                &EventKey::<ChainEv>::of(),
                ChainOnce {
                    hits: hits.clone(),
                    done: done.clone(),
                    value: 4,
                },
            )
            .expect("on");
        ctx.events()
            .emit(&ctx, &EventKey::of(), Arc::new(ChainEv { n: 0 }))
            .expect("emit");
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
        let done = Arc::new(AtomicUsize::new(0));
        ctx.events()
            .on(
                &ctx,
                &EventKey::<ChainEv>::of(),
                ChainOnce {
                    hits: hits.clone(),
                    done: done.clone(),
                    value: 9,
                },
            )
            .expect("on");
        let got = ctx
            .events()
            .waterfall(&ctx, &EventKey::of(), &ChainEv { n: 1 }, ())
            .await
            .expect("waterfall");
        assert_eq!(got, Some(9));
        assert_eq!(hits.load(Ordering::SeqCst), 1);
        let _ = ctx.shutdown().await;
    }
}
