use crate::config::ConfigError;
use crate::localize::Localize;
use crate::output::Output;
use crate::t;
use crate::token::TokenError;
use std::process::ExitCode;
use thiserror::Error;
use tracing::{debug, error};

pub use d1v_api::Error as APIError;

#[derive(Debug, Error)]
pub enum Error {
    /// Not logged in.
    #[error("not logged in")]
    NotLoggedIn,

    /// Token has expired.
    #[error("token expired")]
    TokenExpired,

    /// User canceled the operation (e.g., pressed Esc).
    #[error("canceled")]
    Canceled,

    /// User force-interrupted the operation (e.g., double Ctrl+C).
    #[error("interrupted")]
    Interrupted,

    /// API client error (network, validation, HTTP status, etc.).
    #[error(transparent)]
    Api(APIError),

    /// Configuration error.
    #[error(transparent)]
    Config(#[from] ConfigError),

    /// Token storage error.
    #[error(transparent)]
    Token(#[from] TokenError),

    /// IO error.
    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T = (), E = Error> = std::result::Result<T, E>;

impl From<APIError> for Error {
    fn from(err: APIError) -> Self {
        match err {
            APIError::TokenExpired => Error::TokenExpired,
            other => Error::Api(other),
        }
    }
}

impl Error {
    const EXIT_CANCELED: u8 = 2;
    const EXIT_NETWORK: u8 = 3;
    const EXIT_NOT_LOGGED_IN: u8 = 4;
    /// SIGINT exit code (128 + 2).
    const EXIT_INTERRUPTED: u8 = 130;

    pub fn exit_code(&self) -> ExitCode {
        match self {
            Self::NotLoggedIn | Self::TokenExpired => ExitCode::from(Self::EXIT_NOT_LOGGED_IN),
            Self::Canceled => ExitCode::from(Self::EXIT_CANCELED),
            Self::Interrupted => ExitCode::from(Self::EXIT_INTERRUPTED),
            Self::Api(e) if e.is_network() => ExitCode::from(Self::EXIT_NETWORK),
            _ => ExitCode::FAILURE,
        }
    }

    pub fn hint(&self) -> Option<String> {
        match self {
            Self::NotLoggedIn => Some(t!("hint-not-logged-in")),
            Self::TokenExpired => Some(t!("hint-token-expired")),
            Self::Api(err) if err.is_timeout() => Some(t!("hint-timeout")),
            Self::Api(err) if err.is_connect() => Some(t!("hint-connection")),
            Self::Api(err) if err.is_network() => Some(t!("hint-network")),
            Self::Api(APIError::Url(_)) | Self::Config(_) => Some(t!("hint-config")),
            Self::Token(_) => Some(t!("hint-token-storage")),
            _ => None,
        }
    }

    /// Checks if the error is a user-initiated soft cancellation (e.g., Esc).
    pub fn is_canceled(&self) -> bool {
        matches!(self, Error::Canceled)
    }

    /// Checks if the error is a user-initiated force interruption (e.g., double Ctrl+C).
    pub fn is_interrupted(&self) -> bool {
        matches!(self, Error::Interrupted)
    }

    pub fn handle(&self, output: &Output) -> ExitCode {
        if self.is_canceled() || self.is_interrupted() {
            output.error(&self.localize());
            return self.exit_code();
        }

        output.error(&self.localize());

        if let Some(hint) = self.hint() {
            output.hint(&hint);
        }

        match self {
            Error::Io(_) | Error::Other(_) => error!(%self, "unexpected error"),
            _ => debug!(%self, "cli error"),
        }

        self.exit_code()
    }
}
