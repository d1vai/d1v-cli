use crate::output::Output;
use crate::t;
use std::process::ExitCode;
use thiserror::Error;
use tracing::error;

#[derive(Debug, Error)]
pub enum CliError {
    /// Not logged in.
    #[error("{}", t!("error-not-logged-in"))]
    NotLoggedIn,

    /// User canceled the operation.
    #[error("")]
    Cancelled,
}

impl CliError {
    pub fn exit_code(&self) -> ExitCode {
        match self {
            Self::NotLoggedIn => ExitCode::from(4),
            Self::Cancelled => ExitCode::from(2),
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
        return CliError::Cancelled.exit_code();
    }

    match err.downcast_ref::<CliError>() {
        Some(cli_err) => {
            error!(%err, "fatal error");
            output.error(&err);

            if let Some(hint) = cli_err.hint() {
                output.hint(&hint);
            }

            cli_err.exit_code()
        }
        None => {
            error!(%err, "fatal error");
            output.error(&err);

            ExitCode::FAILURE
        }
    }
}

/// Checks if the error is a user-initiated cancellation.
fn is_cancelled(err: &anyhow::Error) -> bool {
    if err
        .downcast_ref::<CliError>()
        .is_some_and(|e| matches!(e, CliError::Cancelled))
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
