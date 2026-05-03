use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as BASE64;
use jiff::fmt::friendly::{FractionalUnit, SpanPrinter};
use jiff::fmt::serde::timestamp;
use jiff::{SignedDuration, Timestamp};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};

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
    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.expiration_time
            .is_some_and(|time| time <= Timestamp::now())
    }

    pub fn expires_in(&self) -> Option<SignedDuration> {
        self.expiration_time
            .map(|time| time.duration_since(Timestamp::now()))
            .filter(SignedDuration::is_positive)
    }
}

impl Display for Claims {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        static PRINTER: SpanPrinter = SpanPrinter::new()
            .fractional(Some(FractionalUnit::Minute))
            .precision(Some(0));

        if let Some(subject) = &self.subject {
            write!(f, "{subject}")?;
        }

        let separator = if self.subject.as_ref().is_none_or(String::is_empty) {
            ""
        } else {
            " "
        };

        let Some(duration) = self.expires_in() else {
            if self.is_expired() {
                write!(f, "{separator}(expired)")?;
            }

            return Ok(());
        };

        write!(
            f,
            "{separator}(expires in {})",
            PRINTER.duration_to_string(&duration)
        )
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
    let bytes = BASE64.decode(payload)?;

    serde_json::from_slice(&bytes).map_err(DecodeError::Json)
}

/// A JWT bearer token.
#[derive(Clone)]
pub struct Token(SecretString);

impl Token {
    /// Decodes JWT claims without verifying the signature.
    pub fn claims(&self) -> Result<Claims, DecodeError> {
        decode(self.0.expose_secret())
    }

    /// Returns whether the token has expired.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.claims().is_ok_and(|c| c.is_expired())
    }

    /// Returns the remaining validity duration.
    pub fn expires_in(&self) -> Option<SignedDuration> {
        self.claims().ok()?.expires_in()
    }
}

impl ExposeSecret<str> for Token {
    fn expose_secret(&self) -> &str {
        self.0.expose_secret()
    }
}

impl Debug for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl From<SecretString> for Token {
    fn from(secret: SecretString) -> Self {
        Token(secret)
    }
}

impl From<String> for Token {
    fn from(s: String) -> Self {
        Token(SecretString::from(s))
    }
}

impl From<&str> for Token {
    fn from(s: &str) -> Self {
        Token(SecretString::from(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn encode(payload: serde_json::Value) -> String {
        let header = BASE64.encode(json!({"alg": "HS256", "typ": "JWT"}).to_string());
        let payload = BASE64.encode(payload.to_string());

        format!("{header}.{payload}.signature")
    }

    #[test]
    fn decode_valid() {
        let token = encode(json!({
            "sub": "user-123",
            "exp": 9999999999_i64,
            "iat": 1700000000,
        }));
        let claims = decode(&token).unwrap();

        assert_eq!(claims.subject.as_deref(), Some("user-123"));
        assert_eq!(
            claims.expiration_time,
            Timestamp::from_second(9999999999).ok()
        );
        assert_eq!(claims.issued_at, Timestamp::from_second(1700000000).ok());
    }

    #[test]
    fn decode_empty() {
        let token = encode(json!({}));
        let claims = decode(&token).unwrap();

        assert!(claims.subject.is_none());
        assert!(claims.expiration_time.is_none());
        assert!(claims.issued_at.is_none());
    }

    #[test]
    fn decode_unknown_fields() {
        let token = encode(json!({
            "sub": "u",
            "custom_field": "value",
            "nested": {"a": 1},
        }));
        let claims = decode(&token).unwrap();

        assert_eq!(claims.subject.as_deref(), Some("u"));
    }

    #[test]
    fn decode_invalid_token() {
        assert!(matches!(
            decode("not-a-jwt"),
            Err(DecodeError::InvalidToken)
        ));
    }

    #[test]
    fn decode_invalid_base64() {
        assert!(matches!(
            decode("header.!!!invalid!!!.sig"),
            Err(DecodeError::Base64(_))
        ));
    }

    #[test]
    fn decode_invalid_json() {
        let bad_payload = BASE64.encode("not json");
        let token = format!("header.{bad_payload}.sig");

        assert!(matches!(decode(&token), Err(DecodeError::Json(_))));
    }

    #[test]
    fn expired_past() {
        let claims = Claims {
            subject: None,
            expiration_time: Timestamp::from_second(1_000_000_000).ok(),
            issued_at: None,
        };
        assert!(claims.is_expired());
    }

    #[test]
    fn expired_future() {
        let claims = Claims {
            subject: None,
            expiration_time: Some(
                Timestamp::now()
                    .checked_add(SignedDuration::from_hours(1))
                    .unwrap(),
            ),
            issued_at: None,
        };
        assert!(!claims.is_expired());
    }

    #[test]
    fn expired_none() {
        let claims = Claims {
            subject: None,
            expiration_time: None,
            issued_at: None,
        };
        assert!(!claims.is_expired());
    }

    #[test]
    fn expires_in_future() {
        let claims = Claims {
            subject: None,
            expiration_time: Some(
                Timestamp::now()
                    .checked_add(SignedDuration::from_hours(1))
                    .unwrap(),
            ),
            issued_at: None,
        };
        let duration = claims.expires_in().unwrap();
        assert!(duration.as_secs() >= 3598 && duration.as_secs() <= 3600);
    }

    #[test]
    fn expires_in_past() {
        let claims = Claims {
            subject: None,
            expiration_time: Timestamp::from_second(1_000_000_000).ok(),
            issued_at: None,
        };
        assert!(claims.expires_in().is_none());
    }

    #[test]
    fn expires_in_none() {
        let claims = Claims {
            subject: None,
            expiration_time: None,
            issued_at: None,
        };
        assert!(claims.expires_in().is_none());
    }

    #[test]
    fn display() {
        let claims = Claims {
            subject: Some("user-123".into()),
            expiration_time: Some(
                Timestamp::now()
                    .checked_add(SignedDuration::from_hours(2))
                    .unwrap(),
            ),
            issued_at: None,
        };
        let s = claims.to_string();
        assert!(s.starts_with("user-123 (expires in "), "{s}");
    }

    #[test]
    fn display_expired() {
        let claims = Claims {
            subject: Some("user-123".into()),
            expiration_time: Timestamp::from_second(1_000_000_000).ok(),
            issued_at: None,
        };
        assert_eq!(claims.to_string(), "user-123 (expired)");
    }

    #[test]
    fn display_no_exp() {
        let claims = Claims {
            subject: Some("user-123".into()),
            expiration_time: None,
            issued_at: None,
        };
        assert_eq!(claims.to_string(), "user-123");
    }

    #[test]
    fn display_no_sub() {
        let claims = Claims {
            subject: None,
            expiration_time: Timestamp::from_second(1_000_000_000).ok(),
            issued_at: None,
        };
        assert_eq!(claims.to_string(), "(expired)");
    }

    #[test]
    fn display_empty() {
        let claims = Claims {
            subject: None,
            expiration_time: None,
            issued_at: None,
        };
        assert_eq!(claims.to_string(), "");
    }
}
