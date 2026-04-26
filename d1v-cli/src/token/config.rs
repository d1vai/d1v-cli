use secrecy::SecretString;

use super::{Result, TokenError, TokenSource, TokenStore};
use crate::config::Config;

/// Stores token in `~/.d1v/config.toml`.
pub struct ConfigProvider;

impl TokenSource for ConfigProvider {
    fn name(&self) -> &'static str {
        "config"
    }

    fn lookup(&self) -> Result<Option<SecretString>> {
        Ok(Config::load()?.token)
    }
}

impl TokenStore for ConfigProvider {
    fn name(&self) -> &'static str {
        "config"
    }

    fn save(&self, token: &SecretString) -> Result {
        let mut config = Config::load()?;
        config.token = Some(token.clone());
        config.save().map_err(TokenError::Config)
    }

    fn delete(&self) -> Result {
        let mut config = Config::load()?;
        if config.token.is_some() {
            config.token = None;
            config.save()?;
        }

        Ok(())
    }
}
