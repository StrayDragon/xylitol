//! Optional OTLP/HTTP exporter wired through `fastrace-opentelemetry`.
//!
//! Only compiled with Cargo feature `otel`. Callers without the feature treat
//! remote export as always-off (config may still deserialize).

use crate::infra::config::types::{OtelConfig, OtelExporterKind};

/// Resolve whether remote OTLP should be attempted and with which endpoint/headers.
///
/// Returns `None` when export must stay off (default none, missing endpoint, etc.).
pub(crate) fn resolve_otlp_http_target(
    cfg: &OtelConfig,
) -> Option<(String, std::collections::HashMap<String, String>)> {
    resolve_otlp_http_target_with(cfg, |k| std::env::var(k).ok())
}

/// Injectable variant — `get_env` supplies Langfuse base URL / auth keys.
pub(crate) fn resolve_otlp_http_target_with(
    cfg: &OtelConfig,
    get_env: impl Fn(&str) -> Option<String>,
) -> Option<(String, std::collections::HashMap<String, String>)> {
    if !matches!(cfg.exporter, OtelExporterKind::OtlpHttp) {
        return None;
    }

    let endpoint = cfg
        .endpoint
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| endpoint_from_langfuse_env(&get_env))
        .map(normalize_otlp_http_traces_endpoint)?;

    let mut headers = cfg.headers.clone();
    inject_langfuse_auth_headers(&mut headers, &get_env);
    Some((endpoint, headers))
}

/// `opentelemetry-otlp` `.with_endpoint(...)` uses the URL **as-is** and does
/// **not** append `/v1/traces` (unlike `OTEL_EXPORTER_OTLP_ENDPOINT` env).
/// Langfuse base is `…/api/public/otel`; traces land at `…/otel/v1/traces`.
fn normalize_otlp_http_traces_endpoint(endpoint: String) -> String {
    let endpoint = endpoint.trim().trim_end_matches('/').to_string();
    if endpoint.ends_with("/v1/traces") {
        endpoint
    } else {
        format!("{endpoint}/v1/traces")
    }
}

fn endpoint_from_langfuse_env(get_env: impl Fn(&str) -> Option<String>) -> Option<String> {
    let base = get_env("LANGFUSE_BASE_URL")?;
    let base = base.trim().trim_end_matches('/');
    if base.is_empty() {
        return None;
    }
    Some(format!("{base}/api/public/otel"))
}

fn inject_langfuse_auth_headers(
    headers: &mut std::collections::HashMap<String, String>,
    get_env: impl Fn(&str) -> Option<String>,
) {
    let has_auth = headers
        .keys()
        .any(|k| k.eq_ignore_ascii_case("authorization"));
    if !has_auth
        && let (Some(pk), Some(sk)) = (
            get_env("LANGFUSE_PUBLIC_KEY"),
            get_env("LANGFUSE_SECRET_KEY"),
        )
    {
        let pk = pk.trim();
        let sk = sk.trim();
        if !pk.is_empty() && !sk.is_empty() {
            let token = base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                format!("{pk}:{sk}"),
            );
            headers.insert("Authorization".into(), format!("Basic {token}"));
        }
    }
    if !headers
        .keys()
        .any(|k| k.eq_ignore_ascii_case("x-langfuse-ingestion-version"))
        && headers
            .keys()
            .any(|k| k.eq_ignore_ascii_case("authorization"))
    {
        headers.insert("x-langfuse-ingestion-version".into(), "4".into());
    }
}

#[cfg(feature = "otel")]
pub(crate) mod install {
    use std::borrow::Cow;
    use std::future::Future;

    use fastrace::collector::Reporter;
    use fastrace_opentelemetry::OpenTelemetryReporter;
    use opentelemetry::KeyValue;
    use opentelemetry::trace::{SpanContext as OtelSpanContext, TraceFlags};
    use opentelemetry_otlp::{Protocol, SpanExporter, WithExportConfig, WithHttpConfig};
    use opentelemetry_sdk::Resource;
    use opentelemetry_sdk::error::OTelSdkResult;
    use opentelemetry_sdk::trace::{SpanData, SpanExporter as SdkSpanExporter};

    use super::*;
    use crate::infra::config::types::OtelHttpProtocol;

    #[derive(Debug, thiserror::Error)]
    #[error("{0}")]
    struct OtelBuildError(String);

    impl From<String> for OtelBuildError {
        fn from(value: String) -> Self {
            Self(value)
        }
    }

    /// fastrace-opentelemetry 0.18 maps spans with `TraceFlags::default()`
    /// (not sampled). Langfuse (and OTEL exporters generally) drop those.
    /// Force SAMPLED before the OTLP wire encode.
    #[derive(Debug)]
    struct ForceSampledExporter {
        inner: opentelemetry_otlp::SpanExporter,
    }

    impl SdkSpanExporter for ForceSampledExporter {
        fn export(&self, mut batch: Vec<SpanData>) -> impl Future<Output = OTelSdkResult> + Send {
            for span in &mut batch {
                let cx = &span.span_context;
                if !cx.is_sampled() {
                    span.span_context = OtelSpanContext::new(
                        cx.trace_id(),
                        cx.span_id(),
                        TraceFlags::SAMPLED,
                        cx.is_remote(),
                        cx.trace_state().clone(),
                    );
                }
            }
            self.inner.export(batch)
        }

        fn set_resource(&mut self, resource: &Resource) {
            self.inner.set_resource(resource);
        }

        fn shutdown(&self) -> OTelSdkResult {
            self.inner.shutdown()
        }

        fn force_flush(&self) -> OTelSdkResult {
            self.inner.force_flush()
        }
    }

    /// Build an OTLP reporter or `None` on any failure (caller logs + continues).
    pub(crate) fn try_build_otlp_reporter(cfg: &OtelConfig) -> Option<Box<dyn Reporter>> {
        let (endpoint, headers) = resolve_otlp_http_target(cfg)?;
        match build_reporter(cfg, &endpoint, headers) {
            Ok(r) => Some(Box::new(r)),
            Err(e) => {
                log::warn!(
                    target: "xylitol::otel",
                    "OTLP exporter build failed; remote export disabled: {e}"
                );
                None
            }
        }
    }

    fn build_reporter(
        cfg: &OtelConfig,
        endpoint: &str,
        headers: std::collections::HashMap<String, String>,
    ) -> Result<OpenTelemetryReporter, OtelBuildError> {
        let protocol = match cfg.protocol {
            OtelHttpProtocol::HttpBinary => Protocol::HttpBinary,
            OtelHttpProtocol::HttpJson => Protocol::HttpJson,
        };

        // Prefer HTTP/1: some Langfuse self-hosts close HTTP/2 with an empty reply.
        // MUST use async reqwest here: init_logging runs inside the app Tokio
        // runtime; building reqwest::blocking::Client nests/drops another runtime
        // and panics ("Cannot drop a runtime in a context where blocking is not allowed").
        let timeout_secs = cfg.export_timeout_secs.max(1);
        let http_client = reqwest::Client::builder()
            .http1_only()
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .build()
            .map_err(|e| format!("otlp http client: {e}"))?;

        let mut builder = SpanExporter::builder()
            .with_http()
            .with_endpoint(endpoint.to_string())
            .with_protocol(protocol)
            .with_http_client(http_client);
        if !headers.is_empty() {
            builder = builder.with_headers(headers);
        }
        let exporter = builder.build().map_err(|e| format!("span exporter: {e}"))?;
        let exporter = ForceSampledExporter { inner: exporter };

        let service_name = cfg
            .service_name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("xylitol");
        let mut attrs = vec![KeyValue::new("service.name", service_name.to_string())];
        if let Some(env) = cfg
            .environment
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            // Langfuse maps both `deployment.environment` and `langfuse.environment`
            // to the same field — emit the OTel SemConv key only.
            attrs.push(KeyValue::new("deployment.environment", env.to_string()));
        }

        let resource = Resource::builder_empty().with_attributes(attrs).build();
        let scope = opentelemetry::InstrumentationScope::builder("xylitol")
            .with_version(env!("CARGO_PKG_VERSION"))
            .build();

        // Async reqwest needs a Tokio runtime. Prefer the app Handle when the
        // fastrace reporter happens to run inside it; otherwise use a private
        // one-thread runtime (reporter thread has no Handle).
        let reporter = OpenTelemetryReporter::new(exporter, Cow::Owned(resource), scope)
            .with_block_on(otel_block_on);

        log::info!(
            target: "xylitol::otel",
            "OTLP exporter ready endpoint={endpoint} protocol={protocol:?} service={service_name} export_timeout_secs={timeout_secs}"
        );

        Ok(reporter)
    }

    fn otel_block_on(
        future: std::pin::Pin<Box<dyn Future<Output = OTelSdkResult> + Send + '_>>,
    ) -> OTelSdkResult {
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => tokio::task::block_in_place(|| handle.block_on(future)),
            Err(_) => otel_runtime().block_on(future),
        }
    }

    fn otel_runtime() -> &'static tokio::runtime::Runtime {
        static RT: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();
        RT.get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .enable_all()
                .thread_name("xylitol-otel")
                .build()
                .expect("xylitol OTLP tokio runtime")
        })
    }
}

#[cfg(not(feature = "otel"))]
pub(crate) mod install {
    use fastrace::collector::Reporter;

    use super::*;

    pub(crate) fn try_build_otlp_reporter(cfg: &OtelConfig) -> Option<Box<dyn Reporter>> {
        if cfg.wants_otlp_http() {
            log::warn!(
                target: "xylitol::otel",
                "otel config requests otlp-http but binary built without feature `otel`; remote export disabled"
            );
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_exporter_resolves_off() {
        let cfg = OtelConfig::default();
        assert!(resolve_otlp_http_target(&cfg).is_none());
    }

    #[test]
    fn otlp_uses_explicit_endpoint() {
        let cfg = OtelConfig {
            exporter: OtelExporterKind::OtlpHttp,
            endpoint: Some("http://127.0.0.1:3000/api/public/otel".into()),
            ..OtelConfig::default()
        };
        let (ep, _) = resolve_otlp_http_target(&cfg).expect("endpoint");
        assert_eq!(ep, "http://127.0.0.1:3000/api/public/otel/v1/traces");
    }

    #[test]
    fn otlp_without_endpoint_or_env_is_off() {
        let cfg = OtelConfig {
            exporter: OtelExporterKind::OtlpHttp,
            endpoint: None,
            ..OtelConfig::default()
        };
        assert!(resolve_otlp_http_target_with(&cfg, |_| None).is_none());
    }

    #[test]
    fn langfuse_env_derives_endpoint_and_auth() {
        let cfg = OtelConfig {
            exporter: OtelExporterKind::OtlpHttp,
            endpoint: None,
            ..OtelConfig::default()
        };
        let get_env = |k: &str| match k {
            "LANGFUSE_BASE_URL" => Some("http://127.0.0.1:3000/".into()),
            "LANGFUSE_PUBLIC_KEY" => Some("pk-test".into()),
            "LANGFUSE_SECRET_KEY" => Some("sk-test".into()),
            _ => None,
        };
        let (ep, headers) = resolve_otlp_http_target_with(&cfg, get_env).expect("derived");
        assert_eq!(ep, "http://127.0.0.1:3000/api/public/otel/v1/traces");
        assert!(
            headers
                .get("Authorization")
                .is_some_and(|v| v.starts_with("Basic "))
        );
        assert_eq!(
            headers
                .get("x-langfuse-ingestion-version")
                .map(String::as_str),
            Some("4")
        );
    }

    #[test]
    fn normalize_appends_v1_traces_once() {
        assert_eq!(
            normalize_otlp_http_traces_endpoint("http://127.0.0.1:3000/api/public/otel".into()),
            "http://127.0.0.1:3000/api/public/otel/v1/traces"
        );
        assert_eq!(
            normalize_otlp_http_traces_endpoint(
                "http://127.0.0.1:3000/api/public/otel/v1/traces/".into()
            ),
            "http://127.0.0.1:3000/api/public/otel/v1/traces"
        );
    }

    // serial(env_global, obs_global): live smoke 真装 fastrace 全局 reporter（obs_global 必须
    // serial）+ 读 LANGFUSE_* env；#[ignore] 仅显式 live 时跑。消除路径：离线断言走
    // SpanCollectScope/ObsGateScope，live 保持 ignore + serial。
    #[test]
    #[ignore = "live Langfuse; needs LANGFUSE_* + network"]
    #[serial_test::serial(env_global, obs_global)]
    fn live_langfuse_otlp_json_http1_smoke() {
        let cfg = OtelConfig {
            exporter: OtelExporterKind::OtlpHttp,
            protocol: crate::infra::config::types::OtelHttpProtocol::HttpJson,
            environment: Some("dev".into()),
            service_name: Some("xylitol-smoke".into()),
            ..OtelConfig::default()
        };
        let Some(reporter) = install::try_build_otlp_reporter(&cfg) else {
            panic!("expected OTLP reporter (set LANGFUSE_BASE_URL / keys)");
        };
        let mut fanout = crate::infra::observability::FanoutReporter::new(Vec::new());
        fanout.push_box(reporter);
        fastrace::set_reporter(fanout, fastrace::collector::Config::default());
        {
            let _root = fastrace::prelude::Span::root(
                "xylitol.smoke.otlp",
                fastrace::prelude::SpanContext::random(),
            )
            .with_properties(|| {
                [
                    ("langfuse.session.id", "aaagggg".to_string()),
                    ("session.id", "aaagggg".to_string()),
                ]
            });
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        fastrace::flush();
        // Give Langfuse a moment to ingest before the process exits.
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}
