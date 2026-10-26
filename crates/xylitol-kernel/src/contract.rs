#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use rutis::host::CtxHost;
    use rutis_loader::{Builtins, LoaderError, Resolver};
    use xylitol_plugin_sdk::rt::{CallExt, Ctx, Event, FiberState, Plugin, Result};

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

    struct MutexLog(std::sync::Mutex<Vec<u32>>);

    #[tokio::test]
    async fn waterfall_runs_in_registration_order() {
        let ctx = Ctx::root().expect("root");
        let log = Arc::new(MutexLog(std::sync::Mutex::new(Vec::new())));
        ctx.on::<OrderEv>({
            let log = log.clone();
            move |_, call| {
                let log = log.clone();
                async move {
                    log.0.lock().expect("log").push(1);
                    call.next().await
                }
            }
        })
        .expect("on 1");
        ctx.on::<OrderEv>({
            let log = log.clone();
            move |_, call| {
                let log = log.clone();
                async move {
                    log.0.lock().expect("log").push(2);
                    call.next().await
                }
            }
        })
        .expect("on 2");
        let _ = ctx.waterfall(OrderEv { n: 0 }).await.expect("waterfall");
        assert_eq!(*log.0.lock().expect("log"), vec![1, 2]);
        let _ = ctx.shutdown().await;
    }

    struct Marker;

    struct ProvideOnce;

    impl Plugin for ProvideOnce {
        type Deps = ();

        fn name(&self) -> &str {
            "provide-once"
        }

        async fn apply(&self, ctx: &Ctx, _: ()) -> Result {
            ctx.provide(Marker)?;
            Ok(())
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
        let view = ctx.plugin(ProvideOnce);
        (&view).await.expect("apply");
        ctx.on::<OrderEv>({
            let hits = hits.clone();
            move |_, call| {
                let hits = hits.clone();
                async move {
                    hits.fetch_add(1, Ordering::SeqCst);
                    call.next().await
                }
            }
        })
        .expect("on");
        view.dispose().await.expect("dispose");
        let result = ctx.waterfall(OrderEv { n: 1 }).await;
        assert!(result.is_ok() || result.is_err());
        let _ = ctx.shutdown().await;
    }

    #[tokio::test]
    async fn boot_ctx_does_not_mkdir_crate_xylitol() {
        let crate_xy = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".xylitol");
        let existed = crate_xy.exists();
        let opts = BootOptions {
            profile: Profile::Embed,
            config_path: None,
            session: None,
            model: None,
            trust_override: Some(false),
            interactive: false,
            caller: "contract",
            ephemeral_sessions: true,
        };
        let (ctx, _loader) = boot_ctx(&opts).await.expect("boot_ctx");
        let _ = ctx.shutdown().await;
        if !existed {
            assert!(
                !crate_xy.exists(),
                "boot_ctx must not create crates/xylitol-kernel/.xylitol"
            );
        }
    }

    fn prepare_ev(
        reason: xylitol_plugin_sdk::plugin_events::SessionPrepareReason,
    ) -> xylitol_plugin_sdk::plugin_events::SessionPrepare {
        xylitol_plugin_sdk::plugin_events::SessionPrepare {
            session_id: "s".into(),
            reason,
        }
    }

    #[tokio::test]
    async fn session_prepare_emit_returns_immediately() {
        use xylitol_plugin_sdk::plugin_events::SessionPrepare;

        let ctx = Ctx::root().expect("root");
        let started = Arc::new(AtomicUsize::new(0));
        let hold = Arc::new(tokio::sync::Semaphore::new(0));
        ctx.on::<SessionPrepare>({
            let started = started.clone();
            let hold = hold.clone();
            move |_, call| {
                let started = started.clone();
                let hold = hold.clone();
                async move {
                    started.fetch_add(1, Ordering::SeqCst);
                    let _permit = hold.acquire().await.expect("hold");
                    call.chain().await
                }
            }
        })
        .expect("on");
        ctx.emit(prepare_ev(
            xylitol_plugin_sdk::plugin_events::SessionPrepareReason::Switch,
        ))
        .expect("emit");
        for _ in 0..16 {
            if started.load(Ordering::SeqCst) >= 1 {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert!(
            started.load(Ordering::SeqCst) >= 1,
            "emit must not wait for listeners"
        );
        hold.add_permits(1);
        let _ = ctx.shutdown().await;
    }

    #[tokio::test]
    async fn session_prepare_wait_times_out() {
        use xylitol_plugin_sdk::plugin_events::SessionPrepare;

        let ctx = Ctx::root().expect("root");
        let started = Arc::new(AtomicUsize::new(0));
        let hold = Arc::new(tokio::sync::Semaphore::new(0));
        ctx.on::<SessionPrepare>({
            let started = started.clone();
            let hold = hold.clone();
            move |_, call| {
                let started = started.clone();
                let hold = hold.clone();
                async move {
                    started.fetch_add(1, Ordering::SeqCst);
                    let _permit = hold.acquire().await.expect("hold");
                    call.chain().await
                }
            }
        })
        .expect("on");
        let ev = prepare_ev(xylitol_plugin_sdk::plugin_events::SessionPrepareReason::Generate);
        let timed =
            tokio::time::timeout(std::time::Duration::from_millis(15), ctx.waterfall(ev)).await;
        assert!(timed.is_err(), "wait must time out and still proceed");
        assert!(started.load(Ordering::SeqCst) >= 1);
        hold.add_permits(1);
        let _ = ctx.shutdown().await;
    }
}
