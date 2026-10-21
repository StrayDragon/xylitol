use salvo::http::StatusCode;
use serde_json::Value;
use xylitol_protocol::codec;
use xylitol_protocol::{ErrorObject, Outcome, Response};

pub const CONTENT_TYPE: &str = "application/json";

pub const MAX_BODY_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub enum Reply {
    Silent,
    Answer(Value),
    Invalid(Value),
    TooLarge(Value),
}

impl Reply {
    pub fn too_large() -> Self {
        Self::TooLarge(error_only(ErrorObject::payload_too_large()))
    }

    pub fn unreadable() -> Self {
        Self::Invalid(error_only(ErrorObject::parse_error(
            "request body could not be read",
        )))
    }

    pub fn status(&self) -> StatusCode {
        match self {
            Self::Silent | Self::Answer(_) => StatusCode::OK,
            Self::Invalid(_) => StatusCode::BAD_REQUEST,
            Self::TooLarge(_) => StatusCode::PAYLOAD_TOO_LARGE,
        }
    }

    pub fn body(&self) -> Option<String> {
        match self {
            Self::Silent => None,
            Self::Answer(value) | Self::Invalid(value) | Self::TooLarge(value) => {
                Some(value.to_string())
            }
        }
    }
}

fn error_only(error: ErrorObject) -> Value {
    codec::response_value(&Response {
        id: None,
        outcome: Outcome::Error(error),
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn content_type_and_body_limit_are_fixed() {
        assert_eq!(CONTENT_TYPE, "application/json");
        assert_eq!(MAX_BODY_BYTES, 4 * 1024 * 1024);
    }

    #[test]
    fn status_mapping_is_200_400_413() {
        assert_eq!(Reply::Silent.status(), StatusCode::OK);
        assert_eq!(Reply::Answer(json!([])).status(), StatusCode::OK);
        assert_eq!(Reply::Invalid(json!({})).status(), StatusCode::BAD_REQUEST);
        assert_eq!(Reply::too_large().status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[test]
    fn silent_reply_has_no_body() {
        assert_eq!(Reply::Silent.body(), None);
    }

    #[test]
    fn answer_body_is_the_json_value() {
        let value = json!({"jsonrpc": "2.0", "id": "a", "result": {}});
        let body = Reply::Answer(value.clone()).body().expect("answer body");
        assert_eq!(serde_json::from_str::<Value>(&body).expect("json"), value);
    }

    #[test]
    fn too_large_is_the_payload_too_large_envelope() {
        let body = Reply::too_large().body().expect("error body");
        let v: Value = serde_json::from_str(&body).expect("json");
        assert_eq!(
            v,
            json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": {
                    "code": -32600,
                    "message": "payload too large",
                    "data": {"code": "payload_too_large"}
                }
            })
        );
    }

    #[test]
    fn unreadable_body_is_parse_error_400() {
        let reply = Reply::unreadable();
        assert_eq!(reply.status(), StatusCode::BAD_REQUEST);
        let v: Value = serde_json::from_str(&reply.body().expect("body")).expect("json");
        assert_eq!(v["id"], Value::Null);
        assert_eq!(v["error"]["code"], -32700);
    }
}
