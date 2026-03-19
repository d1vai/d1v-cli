use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::{Display, Formatter};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("api error {code}: {message}")]
    Api { code: i64, message: String },

    #[error("invalid response data: {0}")]
    Data(#[from] serde_json::Error),

    #[error(transparent)]
    Validation(#[from] ValidationError),

    #[error(transparent)]
    Url(#[from] url::ParseError),

    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error(transparent)]
    HttpStatus(#[from] HttpStatusError),
}

impl Error {
    pub fn is_api(&self) -> bool {
        matches!(self, Error::Api { .. })
    }

    pub fn api_code(&self) -> Option<i64> {
        match self {
            Error::Api { code, .. } => Some(*code),
            _ => None,
        }
    }

    pub fn is_validation(&self) -> bool {
        matches!(self, Error::Validation(_))
    }

    pub fn is_status(&self) -> bool {
        matches!(self, Error::HttpStatus(_))
    }

    pub fn status_code(&self) -> Option<StatusCode> {
        match self {
            Error::HttpStatus(e) => Some(e.status),
            _ => None,
        }
    }

    pub fn is_network(&self) -> bool {
        matches!(self, Error::Http(_))
    }

    pub fn is_timeout(&self) -> bool {
        matches!(self, Error::Http(e) if e.is_timeout())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Location {
    String(String),
    Integer(i64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationDetail {
    #[serde(rename = "loc")]
    pub location: Vec<Location>,
    #[serde(rename = "msg")]
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
pub struct ValidationError {
    pub detail: Vec<ValidationDetail>,
}

impl Display for ValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "validation errors:")?;

        for ValidationDetail {
            location,
            message,
            error_type,
        } in &self.detail
        {
            let location = location
                .iter()
                .map(|l| match l {
                    Location::String(s) => s.to_string(),
                    Location::Integer(i) => i.to_string(),
                })
                .collect::<Vec<_>>()
                .join(".");

            writeln!(f)?;
            write!(f, "{message} [type={error_type}, location={location}]")?;
        }

        Ok(())
    }
}

#[derive(Debug, Error)]
pub struct HttpStatusError {
    pub status: StatusCode,
    pub body: String,
}

impl HttpStatusError {
    pub fn new(status: StatusCode, body: impl Into<String>) -> Self {
        HttpStatusError {
            status,
            body: body.into(),
        }
    }
}

impl Display for HttpStatusError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "http status error ({})", self.status)?;
        if !self.body.is_empty() {
            write!(f, ": {}", self.body)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_status_error() {
        assert_eq!(
            HttpStatusError::new(StatusCode::NOT_FOUND, "not found").to_string(),
            "http status error (404 Not Found): not found"
        );

        assert_eq!(
            HttpStatusError::new(StatusCode::INTERNAL_SERVER_ERROR, String::new()).to_string(),
            "http status error (500 Internal Server Error)"
        );
    }

    #[test]
    fn validation_error() {
        let json = r#"{
            "detail": [
                {
                    "loc": ["query", "email"],
                    "msg": "Field required",
                    "type": "missing"
                },
                {
                    "loc": ["body", "verify_code"],
                    "msg": "Field required",
                    "type": "missing"
                }
            ]
        }"#;
        let err: ValidationError = serde_json::from_str(json).unwrap();

        assert_eq!(
            err.to_string(),
            concat!(
                "validation errors:\n",
                "Field required [type=missing, location=query.email]\n",
                "Field required [type=missing, location=body.verify_code]"
            )
        );
    }

    #[test]
    fn api_inspection() {
        let err = Error::Api {
            code: 1,
            message: "fail".into(),
        };

        assert!(err.is_api());
        assert_eq!(err.api_code(), Some(1));
        assert!(!err.is_validation());
        assert!(!err.is_status());
        assert!(!err.is_network());
    }

    #[test]
    fn validation_inspection() {
        let err = Error::Validation(ValidationError { detail: vec![] });

        assert!(err.is_validation());
        assert!(!err.is_api());
    }

    #[test]
    fn status_inspection() {
        let err = Error::HttpStatus(HttpStatusError::new(StatusCode::NOT_FOUND, "not found"));

        assert!(err.is_status());
        assert_eq!(err.status_code(), Some(StatusCode::NOT_FOUND));
    }

    #[test]
    fn status_code_returns_none() {
        let err = Error::Api {
            code: 1,
            message: "fail".into(),
        };

        assert_eq!(err.status_code(), None);
    }
}
