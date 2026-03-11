use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::{Display, Formatter};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("api error {code}: {message}")]
    Api { code: i64, message: String },

    #[error("missing data")]
    MissingData,

    #[error(transparent)]
    Validation(#[from] ValidationError),

    #[error(transparent)]
    Url(#[from] url::ParseError),

    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error(transparent)]
    HttpStatus(#[from] HttpStatusError),
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
    fn test_http_status_error() {
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
    fn test_validation_error() {
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
}
