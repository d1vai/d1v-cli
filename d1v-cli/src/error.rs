use crate::t;
use thiserror::Error;

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
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::NotLoggedIn => 4,
            Self::Cancelled => 2,
        }
    }

    pub fn hint(&self) -> Option<String> {
        match self {
            Self::NotLoggedIn => Some(t!("hint-not-logged-in")),
            Self::Cancelled => None,
        }
    }
}
