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

    /// User canceled the operation.
    #[error("cancelled")]
    Cancelled,
}

impl Error {
    const EXIT_NOT_LOGGED_IN: u8 = 4;
    const EXIT_CANCELLED: u8 = 2;

    pub fn exit_code(&self) -> ExitCode {
        match self {
            Self::NotLoggedIn => ExitCode::from(Self::EXIT_NOT_LOGGED_IN),
            Self::Cancelled => ExitCode::from(Self::EXIT_CANCELLED),
        }
    }

    pub fn hint(&self) -> Option<String> {
        match self {
            Self::NotLoggedIn => Some(t!("hint-not-logged-in")),
            Self::Cancelled => None,
        }
    }
}

pub fn handle_error(output: &Output, err: anyhow::Error) -> ExitCode {
    if is_cancelled(&err) {
        output.message(t!("cancelled"));
        return Error::Cancelled.exit_code();
    }

    output.error(&err);

    match err.downcast_ref::<Error>() {
        Some(cli_err) => {
            debug!(%err, "cli error");

            if let Some(hint) = cli_err.hint() {
                output.hint(&hint);
            }

            cli_err.exit_code()
        }
        None => {
            error!(%err, "fatal error");
            ExitCode::FAILURE
        }
    }
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
