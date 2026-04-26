mod chain;
mod config;
mod env;
mod keyring;

pub use chain::TokenChain;

use secrecy::SecretString;
use thiserror::Error;

use crate::config::ConfigError;

type Result<T = (), E = TokenError> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum TokenError {
    #[error("keyring is not available")]
    KeyringUnavailable,

    #[error("failed to load from keyring")]
    KeyringLoad(#[source] keyring_core::Error),

    #[error("failed to save to keyring")]
    KeyringSave(#[source] keyring_core::Error),

    #[error("no writable token store available")]
    NoStore,

    #[error(transparent)]
    Config(#[from] ConfigError),
}

/// A source that can look up authentication tokens.
pub trait TokenSource {
    fn name(&self) -> &'static str;

    /// Looks up a token from this provider.
    fn lookup(&self) -> Result<Option<SecretString>>;
}

/// Persistent storage for authentication tokens.
pub trait TokenStore {
    fn name(&self) -> &'static str;
    fn save(&self, token: &SecretString) -> Result;
    fn delete(&self) -> Result;
}
