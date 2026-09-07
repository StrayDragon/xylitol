//! Tower middleware: portable [`HttpHooks`] ↔ async-openai request factory.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use async_openai::error::OpenAIError;
use async_openai::middleware::HttpRequestFactory;
use reqwest::Response;
use serde_json::Value;
use tower::Service;

use crate::hooks::{
    HeaderBag, HttpHooks, run_after_response, run_before_headers, run_before_request,
};
use crate::provider::obs_session::ObsSessionContext;
use crate::provider::reqwest_bridge::{from_reqwest_headers, to_reqwest_headers};

type BoxFut = Pin<Box<dyn Future<Output = Result<Response, OpenAIError>> + Send>>;

/// Applies xylitol [`HttpHooks`] then executes via reqwest.
#[derive(Clone)]
pub struct HooksHttpService {
    client: reqwest::Client,
    hooks: Option<Arc<dyn HttpHooks>>,
    /// `Some` = generate snapshot (never re-read the process slot).
    /// `None` = idle path (remote count) falls back to process / TLS slot.
    obs_snapshot: Option<ObsSessionContext>,
}

impl HooksHttpService {
    pub fn new(hooks: Option<Arc<dyn HttpHooks>>) -> Self {
        Self::with_snapshot(hooks, None)
    }

    pub fn with_snapshot(
        hooks: Option<Arc<dyn HttpHooks>>,
        obs_snapshot: Option<ObsSessionContext>,
    ) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(crate::provider::native::openai_client::DEFAULT_HTTP_USER_AGENT)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            hooks,
            obs_snapshot,
        }
    }

    /// Per-call wrapping: same reqwest client + hooks, this generate's snapshot.
    pub fn bind_obs(&self, obs: ObsSessionContext) -> Self {
        Self {
            client: self.client.clone(),
            hooks: self.hooks.clone(),
            obs_snapshot: Some(obs),
        }
    }
}

impl Service<HttpRequestFactory> for HooksHttpService {
    type Response = Response;
    type Error = OpenAIError;
    type Future = BoxFut;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, factory: HttpRequestFactory) -> Self::Future {
        let client = self.client.clone();
        let hooks = self.hooks.clone();
        let obs_snapshot = self.obs_snapshot.clone();
        Box::pin(async move {
            let mut request = factory.build().await?;
            apply_hooks_to_request(&hooks, &mut request, obs_snapshot.as_ref()).await?;
            let response = client
                .execute(request)
                .await
                .map_err(OpenAIError::Reqwest)?;
            let status = response.status().as_u16();
            run_after_response(&hooks, status, &from_reqwest_headers(response.headers())).await;
            Ok(response)
        })
    }
}

async fn apply_hooks_to_request(
    hooks: &Option<Arc<dyn HttpHooks>>,
    request: &mut reqwest::Request,
    obs_snapshot: Option<&ObsSessionContext>,
) -> Result<(), OpenAIError> {
    let mut headers: HeaderBag = from_reqwest_headers(request.headers());
    // OpenCode Zen attribution (session + client). Hooks run after and may override.
    match obs_snapshot {
        Some(ctx) => crate::provider::attribution::merge_opencode_attribution_from(
            &mut headers,
            request.url().as_str(),
            ctx,
        ),
        None => crate::provider::attribution::merge_opencode_attribution(
            &mut headers,
            request.url().as_str(),
        ),
    }

    if hooks.is_none() {
        *request.headers_mut() = to_reqwest_headers(&headers);
        return Ok(());
    }

    run_before_headers(hooks, &mut headers)
        .await
        .map_err(|e| OpenAIError::InvalidArgument(e.to_string()))?;
    *request.headers_mut() = to_reqwest_headers(&headers);

    let Some(body) = request.body() else {
        return Ok(());
    };
    let Some(bytes) = body.as_bytes() else {
        return Ok(());
    };
    let mut value: Value = serde_json::from_slice(bytes).map_err(|e| {
        OpenAIError::InvalidArgument(format!("hooks: parse request body JSON: {e}"))
    })?;
    let model = value
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    run_before_request(hooks, &model, &mut value)
        .await
        .map_err(|e| OpenAIError::InvalidArgument(e.to_string()))?;
    let new_bytes = serde_json::to_vec(&value)
        .map_err(|e| OpenAIError::InvalidArgument(format!("hooks: serialize body: {e}")))?;
    *request.body_mut() = Some(reqwest::Body::from(new_bytes));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::obs_session::{ObsSessionScope, set_obs_session};

    #[tokio::test]
    async fn apply_hooks_snapshot_not_process_slot() {
        let _g = ObsSessionScope::enter(ObsSessionContext {
            session_id: Some("process-wrong".into()),
            session_name: None,
            ..Default::default()
        });
        set_obs_session("hijacked", None);
        let mut request = reqwest::Request::new(
            reqwest::Method::POST,
            "https://opencode.ai/zen/v1/chat/completions"
                .parse()
                .expect("url"),
        );
        let snap = ObsSessionContext {
            session_id: Some("bookmark-a".into()),
            session_name: None,
            ..Default::default()
        };
        apply_hooks_to_request(&None, &mut request, Some(&snap))
            .await
            .expect("hooks");
        let got = request
            .headers()
            .get("x-opencode-session")
            .and_then(|v| v.to_str().ok());
        assert_eq!(got, Some("bookmark-a"));
    }
}
