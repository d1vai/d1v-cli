use ahash::AHashMap;
use parking_lot::Mutex;
use reqwest::header::HeaderMap;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// HTTP request metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub method: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "map_is_empty")]
    pub headers: AHashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Value>,
}

impl From<&reqwest::Request> for Request {
    fn from(req: &reqwest::Request) -> Self {
        Request {
            method: req.method().to_string(),
            url: req.url().to_string(),
            headers: collect_headers(req.headers()),
            body: req
                .body()
                .and_then(|b| b.as_bytes())
                .and_then(|bytes| parse_body(bytes)),
        }
    }
}

/// HTTP response metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub status: u16,
    #[serde(default, skip_serializing_if = "map_is_empty")]
    pub headers: AHashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Value>,
}

impl Response {
    /// Creates from raw response parts.
    pub fn new(status: StatusCode, headers: &HeaderMap, body: &[u8]) -> Self {
        Response {
            status: status.as_u16(),
            headers: collect_headers(headers),
            body: parse_body(body),
        }
    }
}

fn parse_body(body: &[u8]) -> Option<Value> {
    if body.is_empty() {
        return None;
    }

    serde_json::from_slice(body)
        .ok()
        .or_else(|| Some(Value::String(String::from_utf8_lossy(body).into_owned())))
}

fn collect_headers(headers: &HeaderMap) -> AHashMap<String, String> {
    headers
        .iter()
        .map(|(name, value)| {
            (
                name.as_str().to_owned(),
                String::from_utf8_lossy(value.as_bytes()).into_owned(),
            )
        })
        .collect()
}

fn map_is_empty<K, V>(map: &AHashMap<K, V>) -> bool {
    map.is_empty()
}

/// A handler for HTTP requests and responses.
pub trait Recorder: Send + Sync {
    fn record(&self, request: &Request, response: &Response);
}

static RECORDER: Mutex<Option<Box<dyn Recorder>>> = Mutex::new(None);

/// Sets the global [`Recorder`], returning a guard that removes it on drop.
pub fn set_recorder(recorder: impl Recorder + 'static) -> Result<RecorderGuard, SetRecorderError> {
    let mut lock = RECORDER.lock();
    if lock.is_some() {
        return Err(SetRecorderError);
    }
    *lock = Some(Box::new(recorder));

    Ok(RecorderGuard)
}

/// A guard that removes the global [`Recorder`] on drop.
#[must_use]
pub struct RecorderGuard;

impl Drop for RecorderGuard {
    fn drop(&mut self) {
        RECORDER.lock().take();
    }
}

#[derive(Debug, thiserror::Error)]
#[error("a global recorder has already been set")]
pub struct SetRecorderError;

/// Dispatches a request/response pair to the global [`Recorder`].
pub(crate) fn dispatch(request: &Request, response: &Response) {
    if let Some(recorder) = RECORDER.lock().as_ref() {
        recorder.record(request, response);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn headers() {
        let req = reqwest::Client::new()
            .post("https://api.example.com/")
            .header("content-type", "application/json")
            .header("authorization", "Bearer secret-token")
            .build()
            .unwrap();

        let req = Request::from(&req);
        assert_eq!(req.headers["content-type"], "application/json");
        assert_eq!(req.headers["authorization"], "Bearer secret-token");
    }

    #[test]
    fn parse_body() {
        assert_eq!(
            super::parse_body(br#"{"code":0,"msg":"success"}"#),
            Some(json!({"code": 0, "msg": "success"}))
        );

        assert_eq!(
            super::parse_body(b"plain text"),
            Some(Value::String("plain text".into()))
        );

        assert_eq!(super::parse_body(b""), None);
    }
}
