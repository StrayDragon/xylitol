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
use crate::provider::reqwest_bridge::{from_reqwest_headers, to_reqwest_headers};

type BoxFut = Pin<Box<dyn Future<Output = Result<Response, OpenAIError>> + Send>>;

/// Applies xylitol [`HttpHooks`] then executes via reqwest.
#[derive(Clone)]
pub struct HooksHttpService {
    client: reqwest::Client,
    hooks: Option<Arc<dyn HttpHooks>>,
}

impl HooksHttpService {
    pub fn new(hooks: Option<Arc<dyn HttpHooks>>) -> Self {
        Self {
            client: reqwest::Client::new(),
            hooks,
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
        Box::pin(async move {
            let mut request = factory.build().await?;
            apply_hooks_to_request(&hooks, &mut request).await?;
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
) -> Result<(), OpenAIError> {
    if hooks.is_none() {
        return Ok(());
    }

    let mut headers: HeaderBag = from_reqwest_headers(request.headers());
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
