use secrecy::{ExposeSecret, SecretString};

use super::{Result, TokenError, TokenSource, TokenStore};
use crate::config::Config;

/// Stores token in `~/.d1v/config.toml`.
pub struct ConfigProvider;

impl TokenSource for ConfigProvider {
    fn name(&self) -> &'static str {
        "config"
    }

    fn lookup(&self) -> Result<Option<SecretString>> {
        let config = Config::load()?;
        Ok(config.api_key.or(config.token))
    }
}

impl TokenStore for ConfigProvider {
    fn name(&self) -> &'static str {
        "config"
    }

    fn save(&self, token: &SecretString) -> Result {
        let mut config = Config::load()?;
        if token.expose_secret().starts_with("sk-") {
            config.api_key = Some(token.clone());
        } else {
            config.token = Some(token.clone());
        }
        config.save().map_err(TokenError::Config)
    }

    fn delete(&self) -> Result {
        let mut config = Config::load()?;
        if config.api_key.is_some() || config.token.is_some() {
            config.api_key = None;
            config.token = None;
            config.save()?;
        }

        Ok(())
    }
}
