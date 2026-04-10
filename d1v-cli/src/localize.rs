use d1v_api::ApiCode;

use crate::config::ConfigError;
use crate::error::{APIError, Error};
use crate::t;
use crate::token::TokenError;

/// Produces a localized, user-facing description of an error.
pub trait Localize {
    fn localize(&self) -> String;
}

impl Localize for Error {
    fn localize(&self) -> String {
        match self {
            Self::NotLoggedIn => t!("error-not-logged-in"),
            Self::TokenExpired => t!("error-token-expired"),
            Self::Canceled => t!("canceled"),
            Self::Config(err) => err.localize(),
            Self::Token(err) => err.localize(),
            Self::Api(err) => err.localize(),
            Self::Io(_) | Self::Other(_) => format!("{self:#}"),
        }
    }
}

impl Localize for ApiCode {
    fn localize(&self) -> String {
        match self {
            ApiCode::PasswordNotSet => t!("api-error-password-not-set"),
            ApiCode::Unknown(code) => t!("api-error-unknown-code", code = code),
            _ => t!("api-error-unknown-code", code = self.raw()),
        }
    }
}

impl Localize for APIError {
    fn localize(&self) -> String {
        match self {
            Self::Http(err) if err.is_timeout() => t!("error-timeout"),
            Self::Http(_) => t!("error-network"),
            Self::HttpStatus(_) => t!("error-http-status"),
            Self::Data(_) => t!("error-invalid-response"),
            Self::Url(_) => t!("error-invalid-url"),
            Self::Validation(_) => t!("error-server-validation"),
            Self::InputValidation(_) => t!("error-input-validation"),
            Self::Api { code, message } => match code {
                ApiCode::Unknown(_) => {
                    t!("api-error-unknown", code = code.raw(), message = message)
                }
                known => known.localize(),
            },
            Self::TokenExpired => t!("error-token-expired"),
        }
    }
}

impl Localize for ConfigError {
    fn localize(&self) -> String {
        match self {
            Self::NoHomeDir => t!("error-no-home-dir"),
            Self::Read(_) => t!("error-read-config"),
            Self::Write(_) => t!("error-write-config"),
            Self::Parse(_) => t!("error-parse-config"),
            Self::Serialize(_) => t!("error-serialize-config"),
        }
    }
}

impl Localize for TokenError {
    fn localize(&self) -> String {
        match self {
            Self::KeyringUnavailable => t!("error-keyring-unavailable"),
            Self::KeyringSave(_) => t!("error-keyring-save"),
            Self::NoStore => t!("error-no-token-store"),
            Self::Config(err) => err.localize(),
        }
    }
}
