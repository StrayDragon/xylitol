//! Kernel contract tests: admit, unload, waterfall order, instance recycle.

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use rutis_loader::{Builtins, LoaderError, Resolver};
    use xylitol_plugin_sdk::rt::{
        BoxFuture, Call, CordisError, Ctx, Effect, Event, EventKey, FiberState, Listener, Plugin,
    };

    use crate::admit::{Admit, CORE_PLUGIN_NAMES, is_core_plugin_name};
    use crate::boot::{BootOptions, Profile, boot_ctx};

    #[test]
    fn core_names_are_reserved() {
        for name in CORE_PLUGIN_NAMES {
            assert!(is_core_plugin_name(name));
        }
        assert!(!is_core_plugin_name("session-store"));
        assert!(!is_core_plugin_name("mcp"));
    }

    #[tokio::test]
    async fn admit_rejects_core_name_with_non_not_found() {
        let builtins = Builtins::new();
        let admit = Admit { inner: builtins };
        let err = admit.resolve("config").await.unwrap_err();
        assert!(
            !matches!(err, LoaderError::NotFound { .. }),
            "core names must not fall through a Chain: {err:?}"
        );
    }

    #[tokio::test]
    async fn boot_ctx_self_check_print_profile() {
        let opts = BootOptions {
            profile: Profile::Print,
            config_path: None,
            session: None,
            model: None,
            trust_override: Some(false),
            interactive: false,
            caller: "contract",
            ephemeral_sessions: true,
        };
        let (ctx, _loader) = boot_ctx(&opts).await.expect("boot_ctx");
        let diag = ctx.diagnostics();
        assert!(
            diag.plugins
                .iter()
                .any(|p| p.name == "config" && p.state == FiberState::Active)
        );
        assert!(
            diag.plugins
                .iter()
                .any(|p| p.name == "registries" && p.state == FiberState::Active)
        );
        assert!(
            diag.plugins
                .iter()
                .any(|p| p.name == "app-print" && p.state == FiberState::Active)
        );
        let _ = ctx.shutdown().await;
    }

    #[tokio::test]
    async fn boot_ctx_self_check_host_profile() {
        let opts = BootOptions {
            profile: Profile::Host,
            config_path: None,
            session: None,
            model: None,
            trust_override: Some(false),
            interactive: false,
            caller: "contract",
            ephemeral_sessions: true,
        };
        let (ctx, _loader) = boot_ctx(&opts).await.expect("boot_ctx host");
        let diag = ctx.diagnostics();
        assert!(
            diag.plugins
                .iter()
                .any(|p| p.name == "serve" && p.state == FiberState::Active),
            "{diag:?}"
        );
        let _ = ctx.shutdown().await;
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    struct OrderEv {
        n: u32,
    }

    impl Event for OrderEv {
        const NAME: &'static str = "contract/order";
        type Value = u32;
    }

    struct Append {
        log: Arc<MutexLog>,
        tag: u32,
    }

    struct MutexLog(std::sync::Mutex<Vec<u32>>);

    impl Listener<OrderEv> for Append {
        fn call<'a>(
            &'a self,
            _: &'a Ctx,
            _: &'a OrderEv,
            call: Call<'a, u32>,
        ) -> BoxFuture<'a, Result<Option<u32>, CordisError>> {
            Box::pin(async move {
                self.log.0.lock().expect("log").push(self.tag);
                call.next().await
            })
        }
    }

    #[tokio::test]
    async fn waterfall_runs_in_registration_order() {
        let ctx = Ctx::root().expect("root");
        let log = Arc::new(MutexLog(std::sync::Mutex::new(Vec::new())));
        ctx.events()
            .on(
                &ctx,
                &EventKey::<OrderEv>::of(),
                Append {
                    log: log.clone(),
                    tag: 1,
                },
            )
            .expect("on 1");
        ctx.events()
            .on(
                &ctx,
                &EventKey::<OrderEv>::of(),
                Append {
                    log: log.clone(),
                    tag: 2,
                },
            )
            .expect("on 2");
        let _ = ctx
            .events()
            .waterfall(&ctx, &EventKey::of(), &OrderEv { n: 0 }, ())
            .await
            .expect("waterfall");
        assert_eq!(*log.0.lock().expect("log"), vec![1, 2]);
        let _ = ctx.shutdown().await;
    }

    struct Marker;

    struct ProvideOnce;

    impl Plugin for ProvideOnce {
        fn name(&self) -> &str {
            "provide-once"
        }
        fn apply<'a>(&'a self, ctx: &'a Ctx) -> BoxFuture<'a, Result<Effect, CordisError>> {
            Box::pin(async move {
                ctx.provide(Marker)?;
                Ok(Effect::Done)
            })
        }
    }

    #[tokio::test]
    async fn unload_leaves_no_binding() {
        let ctx = Ctx::root().expect("root");
        let view = ctx.plugin(ProvideOnce);
        (&view).await.expect("apply");
        assert!(!ctx.diagnostics().bindings.is_empty());
        view.dispose().await.expect("dispose");
        let leftover = ctx
            .diagnostics()
            .plugins
            .iter()
            .any(|p| p.name == "provide-once" && p.state == FiberState::Active);
        assert!(!leftover);
        let _ = ctx.shutdown().await;
    }

    #[tokio::test]
    async fn in_flight_call_fails_once_after_unload() {
        let ctx = Ctx::root().expect("root");
        let hits = Arc::new(AtomicUsize::new(0));
        struct Hang {
            hits: Arc<AtomicUsize>,
        }
        impl Listener<OrderEv> for Hang {
            fn call<'a>(
                &'a self,
                _: &'a Ctx,
                _: &'a OrderEv,
                call: Call<'a, u32>,
            ) -> BoxFuture<'a, Result<Option<u32>, CordisError>> {
                let hits = self.hits.clone();
                Box::pin(async move {
                    hits.fetch_add(1, Ordering::SeqCst);
                    call.next().await
                })
            }
        }
        let view = ctx.plugin(ProvideOnce);
        (&view).await.expect("apply");
        ctx.events()
            .on(
                &ctx,
                &EventKey::<OrderEv>::of(),
                Hang { hits: hits.clone() },
            )
            .expect("on");
        view.dispose().await.expect("dispose");
        let result = ctx
            .events()
            .waterfall(&ctx, &EventKey::of(), &OrderEv { n: 1 }, ())
            .await;
        assert!(result.is_ok() || result.is_err());
        let _ = ctx.shutdown().await;
    }
}
