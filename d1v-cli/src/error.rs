use crate::output::Output;
use crate::t;
use std::process::ExitCode;
use thiserror::Error;
use tracing::{debug, error};

#[derive(Debug, Error)]
pub enum Error {
    /// Not logged in.
    #[error("{}", t!("error-not-logged-in"))]
    NotLoggedIn,

    /// Token has expired.
    #[error("{}", t!("error-token-expired"))]
    TokenExpired,

    /// User canceled the operation.
    #[error("cancelled")]
    Cancelled,
}

impl Error {
    const EXIT_NOT_LOGGED_IN: u8 = 4;
    const EXIT_CANCELLED: u8 = 2;

    pub fn exit_code(&self) -> ExitCode {
        match self {
            Self::NotLoggedIn | Self::TokenExpired => ExitCode::from(Self::EXIT_NOT_LOGGED_IN),
            Self::Cancelled => ExitCode::from(Self::EXIT_CANCELLED),
        }
    }

    pub fn hint(&self) -> Option<String> {
        match self {
            Self::NotLoggedIn => Some(t!("hint-not-logged-in")),
            Self::TokenExpired => Some(t!("hint-token-expired")),
            Self::Cancelled => None,
        }
    }
}

pub fn handle_error(output: &Output, err: anyhow::Error) -> ExitCode {
    if is_cancelled(&err) {
        output.message(t!("cancelled"));
        return Error::Cancelled.exit_code();
    }

    let err = match err.downcast::<d1v_api::Error>() {
        Ok(api_err) if api_err.is_token_expired() => Error::TokenExpired.into(),
        Ok(api_err) => anyhow::Error::from(api_err),
        Err(err) => err,
    };

    output.error(&err);

    let Some(cli_err) = err.downcast_ref::<Error>() else {
        error!(%err, "fatal error");
        return ExitCode::FAILURE;
    };

    debug!(%err, "cli error");
    if let Some(hint) = cli_err.hint() {
        output.hint(&hint);
    }

    cli_err.exit_code()
}

/// Checks if the error is a user-initiated cancellation.
fn is_cancelled(err: &anyhow::Error) -> bool {
    if err
        .downcast_ref::<Error>()
        .is_some_and(|e| matches!(e, Error::Cancelled))
    {
        return true;
    }

    err.downcast_ref::<inquire::InquireError>()
        .is_some_and(|e| {
            matches!(
                e,
                inquire::InquireError::OperationCanceled
                    | inquire::InquireError::OperationInterrupted
            )
        })
}
