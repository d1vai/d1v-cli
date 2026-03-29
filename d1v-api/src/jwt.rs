use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use jiff::fmt::serde::timestamp;
use jiff::{SignedDuration, Timestamp};
use serde::Deserialize;

/// JWT payload claims decoded from a Bearer token.
#[derive(Debug, Clone, Deserialize)]
pub struct Claims {
    #[serde(rename = "sub")]
    pub subject: Option<String>,

    #[serde(default, rename = "exp", with = "timestamp::second::optional")]
    pub expiration_time: Option<Timestamp>,

    #[serde(default, rename = "iat", with = "timestamp::second::optional")]
    pub issued_at: Option<Timestamp>,
}

impl Claims {
    pub fn is_expired(&self) -> bool {
        self.expiration_time
            .is_some_and(|time| time <= Timestamp::now())
    }

    pub fn expires_in(&self) -> Option<SignedDuration> {
        self.expiration_time
            .map(|time| time.duration_since(Timestamp::now()))
            .filter(|duration| duration.is_positive())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("invalid token")]
    InvalidToken,

    #[error("base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),

    #[error("json decode error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Decodes [`Claims`] from a JWT **without** verifying the signature.
pub fn decode(token: impl AsRef<str>) -> Result<Claims, DecodeError> {
    let payload = token
        .as_ref()
        .split('.')
        .nth(1)
        .ok_or(DecodeError::InvalidToken)?;
    let bytes = URL_SAFE_NO_PAD.decode(payload)?;

    serde_json::from_slice(&bytes).map_err(DecodeError::Json)
}
