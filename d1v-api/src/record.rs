use crate::{HttpStatusError, ValidationError};
use ahash::AHashMap;
use reqwest::header::HeaderMap;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::OnceLock;

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

static RECORDER: OnceLock<Box<dyn Recorder>> = OnceLock::new();

/// Sets the global [`Recorder`].
///
/// Returns `Err` if a recorder has already been set.
pub fn set_recorder(recorder: impl Recorder + 'static) -> Result<(), SetRecorderError> {
    RECORDER
        .set(Box::new(recorder))
        .map_err(|_| SetRecorderError)
}

#[derive(Debug, thiserror::Error)]
#[error("a global recorder has already been set")]
pub struct SetRecorderError;

/// Executes a request, dispatches to the global [`Recorder`], and parses the response.
pub(crate) async fn execute(
    http: &reqwest::Client,
    builder: reqwest::RequestBuilder,
) -> Result<crate::response::Response, crate::Error> {
    let request = builder.build()?;
    let recorded_req = Request::from(&request);

    let resp = http.execute(request).await?;
    let status = resp.status();
    let resp_headers = resp.headers().clone();
    let bytes = resp.bytes().await?;

    let recorded_resp = Response::new(status, &resp_headers, &bytes);
    if let Some(recorder) = RECORDER.get() {
        recorder.record(&recorded_req, &recorded_resp);
    }

    match status {
        StatusCode::OK => Ok(serde_json::from_slice(&bytes)?),
        StatusCode::UNPROCESSABLE_ENTITY => {
            Err(serde_json::from_slice::<ValidationError>(&bytes)?.into())
        }
        _ => Err(HttpStatusError::new(status, String::from_utf8_lossy(&bytes)).into()),
    }
}
