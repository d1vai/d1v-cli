use crate::config::ConfigError;
use crate::error::Error;
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
            Self::Config(e) => e.localize(),
            Self::Token(e) => e.localize(),
            _ => format!("{self:#}"),
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
            Self::Config(e) => e.localize(),
        }
    }
}
