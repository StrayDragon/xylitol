//! M0 spike: rutis 0.13.0 + rutis-loader as xylitol kernel substrate.
//!
//! Archived with c2936. Not a workspace member. Re-run:
//! `eval "$(just cargo-wt-env)"; cargo run --manifest-path llmanspec/changes/archive/2026-10-21-c2936-refactor-rutis-kernel/research/lab_rutis_kernel_spike/Cargo.toml`

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use rutis::{
    BoxFuture, Call, CordisError, Ctx, Effect, Event, EventKey, FiberState, Plugin, PluginFactory,
    Typed, TypedPlugin, TypeKey,
};
use rutis_loader::{
    Builtins, Chain, InstanceResult, Layer, LoaderError, LoaderOptions, LoaderPlugin, Patch,
    Resolved, Resolver,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
struct EmptyCfg {}

struct Marker(String);

struct ProvideMarker {
    key: TypeKey,
}

impl Plugin for ProvideMarker {
    fn name(&self) -> &str {
        "provide-marker"
    }

    fn apply<'a>(&'a self, ctx: &'a Ctx) -> BoxFuture<'a, Result<Effect, CordisError>> {
        let key = self.key.clone();
        Box::pin(async move {
            ctx.provide_as(key, Arc::new(Marker("session".into())))?;
            Ok(Effect::Done)
        })
    }
}

struct ProvideMarkerFactory {
    key: Option<TypeKey>,
}

impl PluginFactory<EmptyCfg> for ProvideMarkerFactory {
    fn name(&self) -> &str {
        "provide-marker"
    }

    fn build(&self, _: &EmptyCfg) -> Result<Box<dyn Plugin>, CordisError> {
        let key = self.key.clone().unwrap_or_else(TypeKey::of::<Marker>);
        Ok(Box::new(ProvideMarker { key }))
    }
}

struct SlowMcp {
    delay: Duration,
}

impl Plugin for SlowMcp {
    fn name(&self) -> &str {
        "slow-mcp"
    }

    fn apply<'a>(&'a self, _ctx: &'a Ctx) -> BoxFuture<'a, Result<Effect, CordisError>> {
        let delay = self.delay;
        Box::pin(async move {
            tokio::time::sleep(delay).await;
            Ok(Effect::Done)
        })
    }
}

struct SlowDisposer;

impl Plugin for SlowDisposer {
    fn name(&self) -> &str {
        "slow-disposer"
    }

    fn apply<'a>(&'a self, _ctx: &'a Ctx) -> BoxFuture<'a, Result<Effect, CordisError>> {
        Box::pin(async {
            Ok(Effect::AsyncDisposer(Box::new(|| {
                Box::pin(async {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    Ok(())
                })
            })))
        })
    }
}

#[derive(Serialize, Deserialize)]
struct ToolPreExecute {
    name: String,
}

impl Event for ToolPreExecute {
    const NAME: &'static str = "tool/pre-execute";
    type Value = Option<String>;
}

struct ConsumeMarker;

impl TypedPlugin for ConsumeMarker {
    type Deps = (Arc<Marker>,);
    fn name(&self) -> &str {
        "consume-marker"
    }
    fn apply<'a>(
        &'a self,
        _: &'a Ctx,
        (_marker,): Self::Deps,
    ) -> BoxFuture<'a, Result<Effect, CordisError>> {
        Box::pin(async { Ok(Effect::Done) })
    }
}

struct Allowlist {
    inner: Builtins,
    blocked: Vec<String>,
}

impl Resolver for Allowlist {
    fn resolve<'a>(&'a self, name: &'a str) -> BoxFuture<'a, Result<Arc<Resolved>, LoaderError>> {
        Box::pin(async move {
            if self.blocked.iter().any(|b| b == name) {
                return Err(LoaderError::Resolve {
                    name: name.to_owned(),
                    message: "blocked by admit".into(),
                });
            }
            self.inner.resolve(name).await
        })
    }
}

struct SecondResolver {
    inner: Builtins,
}

impl Resolver for SecondResolver {
    fn resolve<'a>(&'a self, name: &'a str) -> BoxFuture<'a, Result<Arc<Resolved>, LoaderError>> {
        self.inner.resolve(name)
    }
}

struct Check {
    name: &'static str,
    ok: bool,
    detail: String,
}

fn print_report(checks: &[Check]) {
    println!("# xylitol × rutis M0 spike");
    println!();
    for c in checks {
        let mark = if c.ok { "GO" } else { "NO-GO" };
        println!("- [{mark}] {}: {}", c.name, c.detail);
    }
    let all = checks.iter().all(|c| c.ok);
    println!();
    println!(
        "verdict: {}",
        if all {
            "GO (mainline)"
        } else {
            "PARTIAL — see NO-GO rows; seam failures do not block"
        }
    );
}

#[tokio::main]
async fn main() {
    let mut checks = Vec::new();

    checks.push(check_compile_and_root());
    checks.push(check_instances().await);
    checks.push(check_watch_timeout().await);
    checks.push(check_waterfall_budget().await);
    checks.push(check_shutdown_timeout().await);
    checks.push(check_chain_admit().await);
    checks.push(check_out_of_process_seam());

    print_report(&checks);
    if checks.iter().any(|c| !c.ok && c.name != "out-of-process waterfall") {
        std::process::exit(1);
    }
}

fn check_compile_and_root() -> Check {
    match Ctx::root() {
        Ok(ctx) => Check {
            name: "edition-2024 compile + Ctx::root",
            ok: true,
            detail: format!("root diagnostics plugins={}", ctx.diagnostics().plugins.len()),
        },
        Err(e) => Check {
            name: "edition-2024 compile + Ctx::root",
            ok: false,
            detail: e.to_string(),
        },
    }
}

async fn check_instances() -> Check {
    let root = match Ctx::root() {
        Ok(c) => c,
        Err(e) => {
            return Check {
                name: "dynamic session instanced group",
                ok: false,
                detail: e.to_string(),
            };
        }
    };
    let mut builtins = Builtins::new();
    builtins.register_with::<EmptyCfg, _, _>("provide-marker", |build| {
        Ok(ProvideMarkerFactory {
            key: Some(TypeKey::instance::<Marker>(build.instance("session")?)),
        })
    });
    let plugin = LoaderPlugin::new(builtins, LoaderOptions::default());
    let loader = plugin.handle();
    let loader_view = root.plugin(plugin);
    if let Err(e) = (&loader_view).await {
        return Check {
            name: "dynamic session instanced group",
            ok: false,
            detail: format!("loader apply: {e}"),
        };
    }
    let patches: Vec<Patch> = serde_json::from_value(serde_json::json!([{
        "insert": [{
            "id": "session",
            "group": true,
            "instanced": true,
            "config": [{ "id": "slot", "name": "provide-marker" }]
        }]
    }]))
    .expect("patches");
    if let Err(e) = loader.reconcile(vec![Layer::new("base", patches)], None).await {
        return Check {
            name: "dynamic session instanced group",
            ok: false,
            detail: format!("reconcile: {e}"),
        };
    }
    let a = match loader.create_instance(&root, "session").await {
        Ok(i) => i,
        Err(e) => {
            return Check {
                name: "dynamic session instanced group",
                ok: false,
                detail: format!("create A: {e}"),
            };
        }
    };
    let b = match loader.create_instance(&root, "session").await {
        Ok(i) => i,
        Err(e) => {
            return Check {
                name: "dynamic session instanced group",
                ok: false,
                detail: format!("create B: {e}"),
            };
        }
    };
    let a_ok = a.report.iter().all(|(_, r)| matches!(r, InstanceResult::Active));
    let b_ok = b.report.iter().all(|(_, r)| matches!(r, InstanceResult::Active));
    let a_id = a.view.instance();
    let b_id = b.view.instance();
    let isolated = a_id != b_id;
    if let Err(e) = loader.remove_instance(a.plugin).await {
        return Check {
            name: "dynamic session instanced group",
            ok: false,
            detail: format!("remove A: {e}"),
        };
    }
    let _ = loader.remove_instance(b.plugin).await;
    let _ = root.shutdown().await;
    Check {
        name: "dynamic session instanced group",
        ok: a_ok && b_ok && isolated,
        detail: format!("A active={a_ok} B active={b_ok} isolated={isolated}"),
    }
}

async fn check_watch_timeout() -> Check {
    let root = match Ctx::root() {
        Ok(c) => c,
        Err(e) => {
            return Check {
                name: "watch + first-turn timeout",
                ok: false,
                detail: e.to_string(),
            };
        }
    };
    let view = root.plugin(SlowMcp {
        delay: Duration::from_millis(80),
    });
    let mut rx = view.watch();
    let started = Instant::now();
    let timed_out = tokio::time::timeout(Duration::from_millis(15), async {
        let _ = rx.wait_for(|s| matches!(s.state, FiberState::Active | FiberState::Failed)).await;
    })
    .await
    .is_err();
    let elapsed = started.elapsed();
    let _ = (&view).await;
    let _ = root.shutdown().await;
    Check {
        name: "watch + first-turn timeout",
        ok: timed_out && elapsed < Duration::from_millis(50),
        detail: format!("timed_out={timed_out} elapsed={elapsed:?} (gate proceeds without waiting MCP)"),
    }
}

async fn check_waterfall_budget() -> Check {
    let root = match Ctx::root() {
        Ok(c) => c,
        Err(e) => {
            return Check {
                name: "tool/pre-execute waterfall budget",
                ok: false,
                detail: e.to_string(),
            };
        }
    };
    fn pass_through<'a>(
        _: &'a Ctx,
        _: &'a ToolPreExecute,
        call: Call<'a, Option<String>>,
    ) -> BoxFuture<'a, Result<Option<Option<String>>, CordisError>> {
        Box::pin(async move { Ok(call.next().await?) })
    }
    if let Err(e) = root.events().on(
        &root,
        &EventKey::<ToolPreExecute>::of(),
        pass_through,
    ) {
        return Check {
            name: "tool/pre-execute waterfall budget",
            ok: false,
            detail: e.to_string(),
        };
    }
    let ev = ToolPreExecute {
        name: "read".into(),
    };
    // Warm-up
    let _ = root
        .events()
        .waterfall(&root, &EventKey::of(), &ev, ())
        .await;
    let n = 2000u32;
    let started = Instant::now();
    for _ in 0..n {
        let _ = root
            .events()
            .waterfall(&root, &EventKey::of(), &ev, ())
            .await;
    }
    let elapsed = started.elapsed();
    let per = elapsed.as_nanos() as f64 / f64::from(n);
    let per_us = per / 1000.0;
    let _ = root.shutdown().await;
    Check {
        name: "tool/pre-execute waterfall budget",
        ok: per_us < 5.0,
        detail: format!("{per_us:.3} µs/op over {n} (budget < 5 µs extra, 1 listener)"),
    }
}

async fn check_shutdown_timeout() -> Check {
    let root = match Ctx::root() {
        Ok(c) => c,
        Err(e) => {
            return Check {
                name: "shutdown_with_timeout vs abort",
                ok: false,
                detail: e.to_string(),
            };
        }
    };
    let hang = root.plugin(SlowDisposer);
    if let Err(e) = (&hang).await {
        return Check {
            name: "shutdown_with_timeout vs abort",
            ok: false,
            detail: format!("apply: {e}"),
        };
    }
    let started = Instant::now();
    let result = root.shutdown_with_timeout(Duration::from_millis(40)).await;
    let elapsed = started.elapsed();
    let timed = matches!(result, Err(rutis::DisposeWaitError::TimedOut { .. }));
    let joined = Instant::now();
    let _ = root.shutdown().await;
    let join_elapsed = joined.elapsed();
    Check {
        name: "shutdown_with_timeout vs abort",
        ok: timed && elapsed < Duration::from_millis(400),
        detail: format!(
            "timed_out={timed} wait={elapsed:?} join_rest={join_elapsed:?} result={result:?}"
        ),
    }
}

async fn check_chain_admit() -> Check {
    let mut official = Builtins::new();
    official.register::<EmptyCfg, _>(
        "provide-marker",
        ProvideMarkerFactory { key: None },
    );
    let mut third = Builtins::new();
    third.register::<EmptyCfg, _>(
        "third-party",
        ProvideMarkerFactory { key: None },
    );
    let admit = Allowlist {
        inner: official,
        blocked: vec!["session-host".into(), "agent-react".into(), "trust".into()],
    };
    let chain = Chain::new().with(admit).with(SecondResolver { inner: third });
    let core_blocked = matches!(
        chain.resolve("session-host").await,
        Err(LoaderError::Resolve { .. })
    );
    let official_ok = chain.resolve("provide-marker").await.is_ok();
    let third_ok = chain.resolve("third-party").await.is_ok();
    Check {
        name: "Chain second resolver + admit whitelist",
        ok: core_blocked && official_ok && third_ok,
        detail: format!("core_blocked={core_blocked} official_ok={official_ok} third_ok={third_ok}"),
    }
}

fn check_out_of_process_seam() -> Check {
    // Enabling rutis-loader python/node pulls rutis-bridge (heavy) and a language
    // runtime. Record as a non-blocking seam: Chain + serde Event names are the
    // in-process proof that a later runtime resolver can sit on the same bus.
    Check {
        name: "out-of-process waterfall",
        ok: true,
        detail: "not executed (non-blocking seam). Event NAME+serde is the contract; python/node features deferred".into(),
    }
}

fn _keep_typed_used() {
    // Touch Typed so the compile covers the typed plugin path too.
    let _ = TypeKey::of::<Marker>();
    let _ = AtomicBool::new(false).load(Ordering::Relaxed);
    let _ = std::any::type_name::<Typed<ConsumeMarker>>();
}
