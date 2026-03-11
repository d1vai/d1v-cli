use crate::ValidationError;
use reqwest::StatusCode;
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
    HttpStatus(HttpStatusError),
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
}
