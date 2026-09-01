use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::{Display, Formatter};
use thiserror::Error;

use crate::validate::{CodeError, EmailError, UrlError, ValidationError};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ApiCode {
    /// A common request error code.
    BadRequest,
    /// Authentication required (HTTP 401).
    Unauthorized,
    /// Permission denied (HTTP 403).
    Forbidden,
    Unknown(i64),
}

/// Specific cause of an [`ApiCode::BadRequest`] response.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum BadRequestKind {
    PasswordNotSet,
    InvalidCredentials,
    EmailRequiredBeforePassword,
    InvalidVerifyCode,
    VerifyCodeExpired,
    InvalidOrExpiredCode,
    UserNotFound,
    PasswordTooShort,
    EmailAlreadyInUse,
    EmailNotBound,
    CannotAcceptOwnInviteCode,
    InvalidInviteCode,
    InviteCodeExpired,
    InviteCodeCapacityReached,
    InviteLimitReached,
    InviteCodeNotBoundToInviter,
    InviterNotFound,
}

impl BadRequestKind {
    /// Returns the kind matching `message`, or `None` if unrecognized.
    #[must_use]
    pub fn from_message(message: &str) -> Option<Self> {
        Some(match message {
            "password not set" => Self::PasswordNotSet,
            "invalid email or password" => Self::InvalidCredentials,
            "email is required before setting a password" => Self::EmailRequiredBeforePassword,
            "invalid verify code" => Self::InvalidVerifyCode,
            "verify code expired" => Self::VerifyCodeExpired,
            "invalid or expired code" => Self::InvalidOrExpiredCode,
            "user not found" => Self::UserNotFound,
            "password too short" => Self::PasswordTooShort,
            "email already in use" => Self::EmailAlreadyInUse,
            "email not bound" => Self::EmailNotBound,
            "cannot accept your own invite code" => Self::CannotAcceptOwnInviteCode,
            "invalid invite code" => Self::InvalidInviteCode,
            "invite code expired" => Self::InviteCodeExpired,
            "invite code capacity reached" => Self::InviteCodeCapacityReached,
            "invite limit reached for this code" => Self::InviteLimitReached,
            "invite code not bound to inviter" => Self::InviteCodeNotBoundToInviter,
            "inviter not found" => Self::InviterNotFound,
            _ => return None,
        })
    }

    /// Canonical message for this kind.
    #[must_use]
    pub fn message(&self) -> &'static str {
        match self {
            Self::PasswordNotSet => "password not set",
            Self::InvalidCredentials => "invalid email or password",
            Self::EmailRequiredBeforePassword => "email is required before setting a password",
            Self::InvalidVerifyCode => "invalid verify code",
            Self::VerifyCodeExpired => "verify code expired",
            Self::InvalidOrExpiredCode => "invalid or expired code",
            Self::UserNotFound => "user not found",
            Self::PasswordTooShort => "password too short",
            Self::EmailAlreadyInUse => "email already in use",
            Self::EmailNotBound => "email not bound",
            Self::CannotAcceptOwnInviteCode => "cannot accept your own invite code",
            Self::InvalidInviteCode => "invalid invite code",
            Self::InviteCodeExpired => "invite code expired",
            Self::InviteCodeCapacityReached => "invite code capacity reached",
            Self::InviteLimitReached => "invite limit reached for this code",
            Self::InviteCodeNotBoundToInviter => "invite code not bound to inviter",
            Self::InviterNotFound => "inviter not found",
        }
    }
}

impl ApiCode {
    #[must_use]
    pub fn raw(&self) -> i64 {
        match self {
            Self::BadRequest => 40000,
            Self::Unauthorized => 401,
            Self::Forbidden => 403,
            Self::Unknown(code) => *code,
        }
    }
}

impl From<i64> for ApiCode {
    fn from(code: i64) -> Self {
        match code {
            40000 => Self::BadRequest,
            401 => Self::Unauthorized,
            403 => Self::Forbidden,
            _ => Self::Unknown(code),
        }
    }
}

impl From<ApiCode> for i64 {
    fn from(code: ApiCode) -> Self {
        code.raw()
    }
}

impl Display for ApiCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.raw())
    }
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("api error {code}: {message}")]
pub struct ApiError {
    pub code: ApiCode,
    pub message: String,
}

impl ApiError {
    pub fn new(code: impl Into<ApiCode>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    /// Returns the [`BadRequestKind`] when this is a recognized [`ApiCode::BadRequest`] response.
    #[must_use]
    pub fn bad_request_kind(&self) -> Option<BadRequestKind> {
        if self.code == ApiCode::BadRequest {
            BadRequestKind::from_message(&self.message)
        } else {
            None
        }
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Api(#[from] ApiError),

    #[error("response data schema mismatch: {0}")]
    Data(#[from] serde_json::Error),

    #[error(transparent)]
    ServerValidation(#[from] ServerValidationError),

    #[error(transparent)]
    Url(#[from] url::ParseError),

    #[error(transparent)]
    Validation(#[from] ValidationError),

    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error(transparent)]
    HttpStatus(#[from] HttpStatusError),

    #[error("token expired")]
    TokenExpired,
}

impl Error {
    pub fn api(code: impl Into<ApiCode>, message: impl Into<String>) -> Self {
        Error::Api(ApiError::new(code, message))
    }

    #[must_use]
    pub fn is_api(&self) -> bool {
        matches!(self, Error::Api(_))
    }

    pub fn api_code(&self) -> Option<ApiCode> {
        if let Error::Api(err) = self {
            Some(err.code)
        } else {
            None
        }
    }

    /// Returns the [`BadRequestKind`] for a recognized [`ApiCode::BadRequest`] message.
    #[must_use]
    pub fn bad_request_kind(&self) -> Option<BadRequestKind> {
        if let Error::Api(err) = self {
            err.bad_request_kind()
        } else {
            None
        }
    }

    #[must_use]
    pub fn is_server_validation(&self) -> bool {
        matches!(self, Error::ServerValidation(_))
    }

    #[must_use]
    pub fn is_status(&self) -> bool {
        matches!(self, Error::HttpStatus(_))
    }

    pub fn status_code(&self) -> Option<StatusCode> {
        match self {
            Error::HttpStatus(e) => Some(e.status),
            _ => None,
        }
    }

    #[must_use]
    pub fn is_network(&self) -> bool {
        matches!(self, Error::Http(_))
    }

    #[must_use]
    pub fn is_connect(&self) -> bool {
        matches!(self, Error::Http(e) if e.is_connect())
    }

    #[must_use]
    pub fn is_validation(&self) -> bool {
        matches!(self, Error::Validation(_))
    }

    #[must_use]
    pub fn is_timeout(&self) -> bool {
        matches!(self, Error::Http(e) if e.is_timeout())
    }

    #[must_use]
    pub fn is_token_expired(&self) -> bool {
        matches!(self, Error::TokenExpired)
    }
}

impl From<EmailError> for Error {
    fn from(err: EmailError) -> Self {
        Error::Validation(err.into())
    }
}

impl From<CodeError> for Error {
    fn from(err: CodeError) -> Self {
        Error::Validation(err.into())
    }
}

impl From<UrlError> for Error {
    fn from(err: UrlError) -> Self {
        Error::Validation(err.into())
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
pub struct ServerValidationError {
    pub detail: Vec<ValidationDetail>,
}

impl Display for ServerValidationError {
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
                    Location::String(s) => s.clone(),
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
        let err: ServerValidationError = serde_json::from_str(json).unwrap();

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
        let err = Error::api(1, "fail");

        assert!(err.is_api());
        assert_eq!(err.api_code(), Some(ApiCode::Unknown(1)));
        assert!(!err.is_server_validation());
        assert!(!err.is_status());
        assert!(!err.is_network());
    }

    #[test]
    fn bad_request_api_code() {
        assert_eq!(ApiCode::from(40000), ApiCode::BadRequest);
        assert_eq!(ApiCode::BadRequest.raw(), 40000);
        assert_eq!(i64::from(ApiCode::BadRequest), 40000);
    }

    #[test]
    fn auth_api_codes() {
        assert_eq!(ApiCode::from(401), ApiCode::Unauthorized);
        assert_eq!(ApiCode::from(403), ApiCode::Forbidden);
        assert_eq!(ApiCode::Unauthorized.raw(), 401);
        assert_eq!(ApiCode::Forbidden.raw(), 403);
    }

    #[test]
    fn bad_request_kind() {
        for kind in [
            BadRequestKind::PasswordNotSet,
            BadRequestKind::InvalidCredentials,
            BadRequestKind::EmailRequiredBeforePassword,
            BadRequestKind::InvalidVerifyCode,
            BadRequestKind::VerifyCodeExpired,
            BadRequestKind::InvalidOrExpiredCode,
            BadRequestKind::UserNotFound,
            BadRequestKind::PasswordTooShort,
            BadRequestKind::EmailAlreadyInUse,
            BadRequestKind::EmailNotBound,
            BadRequestKind::CannotAcceptOwnInviteCode,
            BadRequestKind::InvalidInviteCode,
            BadRequestKind::InviteCodeExpired,
            BadRequestKind::InviteCodeCapacityReached,
            BadRequestKind::InviteLimitReached,
            BadRequestKind::InviteCodeNotBoundToInviter,
            BadRequestKind::InviterNotFound,
        ] {
            assert_eq!(BadRequestKind::from_message(kind.message()), Some(kind));
        }
        assert_eq!(BadRequestKind::from_message("something new"), None);
        assert_eq!(BadRequestKind::from_message(""), None);
    }

    #[test]
    fn bad_request_kind_from_error() {
        let err = Error::api(ApiCode::BadRequest, "password not set");
        assert_eq!(err.bad_request_kind(), Some(BadRequestKind::PasswordNotSet));

        let err = Error::api(ApiCode::BadRequest, "something unknown");
        assert_eq!(err.bad_request_kind(), None);

        let err = Error::api(ApiCode::Unknown(500), "password not set");
        assert_eq!(err.bad_request_kind(), None);

        let err = Error::TokenExpired;
        assert_eq!(err.bad_request_kind(), None);
    }

    #[test]
    fn validation_inspection() {
        let err = Error::ServerValidation(ServerValidationError { detail: vec![] });

        assert!(err.is_server_validation());
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
        let err = Error::api(1, "fail");

        assert_eq!(err.status_code(), None);
    }
}
