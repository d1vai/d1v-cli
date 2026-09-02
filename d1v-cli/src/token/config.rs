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
        set_token(&mut config, token);
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

fn set_token(config: &mut Config, token: &SecretString) {
    config.api_key = None;
    config.token = None;
    if token.expose_secret().starts_with("sk-") {
        config.api_key = Some(token.clone());
    } else {
        config.token = Some(token.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::set_token;
    use crate::config::Config;
    use secrecy::{ExposeSecret, SecretString};

    #[test]
    fn replacing_api_key_clears_stale_token() {
        let mut config = Config::default();
        config.token = Some(SecretString::from("old-token"));
        set_token(&mut config, &SecretString::from("sk-new"));
        assert!(config.token.is_none());
        assert_eq!(config.api_key.unwrap().expose_secret(), "sk-new");
    }

    #[test]
    fn replacing_token_clears_stale_api_key() {
        let mut config = Config::default();
        config.api_key = Some(SecretString::from("sk-old"));
        set_token(&mut config, &SecretString::from("new-token"));
        assert!(config.api_key.is_none());
        assert_eq!(config.token.unwrap().expose_secret(), "new-token");
    }
}
