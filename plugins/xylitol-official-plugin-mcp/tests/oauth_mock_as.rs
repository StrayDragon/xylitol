use rmcp::transport::auth::{AuthorizationRequest, OAuthState};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use url::Url;

async fn mock_as() -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let issuer = format!("http://{addr}");
    let task = tokio::spawn({
        let issuer = issuer.clone();
        async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    return;
                };
                let mut buf = vec![0u8; 16 * 1024];
                let Ok(read) = stream.read(&mut buf).await else {
                    continue;
                };
                let request = String::from_utf8_lossy(&buf[..read]);
                let path = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .unwrap_or("/");
                let body = if path.starts_with("/.well-known/oauth-authorization-server") {
                    format!(
                        r#"{{"issuer":"{issuer}","authorization_endpoint":"{issuer}/authorize","token_endpoint":"{issuer}/token","response_types_supported":["code"],"grant_types_supported":["authorization_code","refresh_token"],"code_challenge_methods_supported":["S256"],"token_endpoint_auth_methods_supported":["none"]}}"#
                    )
                } else if path.starts_with("/token") {
                    r#"{"access_token":"test-access","token_type":"Bearer","expires_in":3600,"refresh_token":"test-refresh","scope":"read"}"#.to_string()
                } else {
                    r#"{"error":"not_found"}"#.to_string()
                };
                let status = if path.starts_with("/token") || path.starts_with("/.well-known") {
                    "200 OK"
                } else {
                    "404 Not Found"
                };
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes()).await;
            }
        }
    });
    (issuer, task)
}

#[tokio::test]
async fn oauth_state_round_trip_against_mock_as() {
    let (issuer, server) = mock_as().await;
    let http_client = reqwest::Client::new();
    let mut state = OAuthState::new(issuer.clone(), Some(http_client.clone()))
        .await
        .expect("state");
    let OAuthState::Unauthorized(manager) = &mut state else {
        panic!("expected unauthorized");
    };
    manager.set_credential_store(
        xylitol_official_plugin_mcp::credential_store::OAuthCredentialStore::new(
            &std::env::temp_dir().join(format!("xylitol-oauth-test-{}", std::process::id())),
            "mock-as",
            xylitol_official_plugin_mcp::McpOAuthKeychain::Off,
        ),
    );
    let redirect = format!("{issuer}/callback");
    state
        .start_authorization(
            AuthorizationRequest::new(redirect.clone())
                .with_client_name("xylitol")
                .with_preregistered_client("test-client")
                .with_scopes(["read"]),
        )
        .await
        .expect("start authorization");
    let auth_url = state.get_authorization_url().await.expect("auth url");
    let parsed = Url::parse(&auth_url).expect("auth url parse");
    let query: std::collections::HashMap<_, _> = parsed.query_pairs().into_owned().collect();
    assert_eq!(
        query.get("code_challenge_method").map(String::as_str),
        Some("S256"),
        "PKCE S256 challenge required"
    );
    assert!(
        query.get("code_challenge").is_some_and(|v| !v.is_empty()),
        "PKCE challenge required"
    );
    let state_param = query.get("state").cloned().expect("state");
    let callback = format!("{redirect}?code=test-code&state={state_param}&iss={issuer}");
    state
        .handle_callback_url(&callback)
        .await
        .expect("callback");
    let manager = state
        .into_authorization_manager()
        .expect("authorized manager");
    assert!(manager.get_access_token().await.is_ok());
    server.abort();
}
