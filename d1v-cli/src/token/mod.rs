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

    #[error("failed to save to keyring")]
    KeyringSave(#[source] keyring_core::Error),

    #[error("no writable token store available")]
    NoStore,

    #[error(transparent)]
    Config(#[from] ConfigError),
}

/// A source that provides authentication tokens.
pub trait TokenLoader {
    fn name(&self) -> &'static str;
    fn load(&self) -> Result<Option<SecretString>>;
}

/// Persistent storage for authentication tokens.
pub trait TokenStore {
    fn name(&self) -> &'static str;
    fn save(&self, token: &SecretString) -> Result;
    fn delete(&self) -> Result;
}

const KEYRING_SERVICE: &str = "d1v-cli";
const KEYRING_USER: &str = "token";
