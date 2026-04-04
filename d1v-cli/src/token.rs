use anyhow::{bail, Context, Result};
use secrecy::{ExposeSecret, SecretString};
use tracing::{debug, warn};

use crate::config::Config;
use crate::t;

/// A source that provides authentication tokens.
pub trait TokenLoader {
    fn name(&self) -> &str;
    fn load(&self) -> Result<Option<SecretString>>;
}

/// Persistent storage for authentication tokens.
pub trait TokenStore {
    fn name(&self) -> &str;
    fn save(&self, token: &SecretString) -> Result<()>;
    fn delete(&self) -> Result<()>;
}

/// Reads token from an environment variable.
pub struct EnvProvider {
    var_name: &'static str,
}

impl EnvProvider {
    pub fn new(var_name: &'static str) -> Self {
        Self { var_name }
    }
}

impl TokenLoader for EnvProvider {
    fn name(&self) -> &str {
        self.var_name
    }

    fn load(&self) -> Result<Option<SecretString>> {
        match std::env::var(self.var_name) {
            Ok(v) if !v.is_empty() => Ok(Some(SecretString::from(v))),
            _ => Ok(None),
        }
    }
}

/// Stores token in the OS keychain.
pub struct KeyringProvider {
    service: &'static str,
    user: &'static str,
}

impl KeyringProvider {
    pub fn new(service: &'static str, user: &'static str) -> Self {
        Self { service, user }
    }

    fn entry(&self) -> Option<keyring::Entry> {
        keyring::Entry::new(self.service, self.user).ok()
    }
}

impl TokenLoader for KeyringProvider {
    fn name(&self) -> &str {
        "keyring"
    }

    fn load(&self) -> Result<Option<SecretString>> {
        if let Some(entry) = self.entry()
            && let Ok(password) = entry.get_password()
        {
            return Ok(Some(SecretString::from(password)));
        }

        Ok(None)
    }
}

impl TokenStore for KeyringProvider {
    fn name(&self) -> &str {
        "keyring"
    }

    fn save(&self, token: &SecretString) -> Result<()> {
        let entry = self.entry().context(t!("error-keyring-unavailable"))?;
        entry
            .set_password(token.expose_secret())
            .context(t!("error-keyring-save"))
    }

    fn delete(&self) -> Result<()> {
        let Some(entry) = self.entry() else {
            return Ok(());
        };

        match entry.delete_credential() {
            Ok(()) => {}
            Err(keyring::Error::NoEntry) => {
                debug!("no keyring credential to delete");
            }
            Err(err) => {
                warn!(error = %err, "failed to delete keyring credential");
            }
        }

        Ok(())
    }
}

/// Stores token in `~/.d1v/config.toml`.
pub struct ConfigProvider;

impl TokenLoader for ConfigProvider {
    fn name(&self) -> &str {
        "config"
    }

    fn load(&self) -> Result<Option<SecretString>> {
        let config = Config::load()?;

        Ok(config.token)
    }
}

impl TokenStore for ConfigProvider {
    fn name(&self) -> &str {
        "config"
    }

    fn save(&self, token: &SecretString) -> Result<()> {
        let mut config = Config::load()?;
        config.token = Some(token.clone());
        config.save()?;
        Ok(())
    }

    fn delete(&self) -> Result<()> {
        let mut config = Config::load()?;
        if config.token.is_some() {
            config.token = None;
            config.save()?;
        }

        Ok(())
    }
}

/// Chains multiple providers in priority order.
///
/// `load` returns the first token found;
/// `save` writes to the first store that succeeds;
/// `delete` removes from all stores.
pub struct TokenChain {
    loaders: Vec<Box<dyn TokenLoader>>,
    stores: Vec<Box<dyn TokenStore>>,
}

impl TokenChain {
    pub fn new(loaders: Vec<Box<dyn TokenLoader>>, stores: Vec<Box<dyn TokenStore>>) -> Self {
        Self { loaders, stores }
    }

    /// Returns the name of the first loader that provides a token.
    pub fn source(&self) -> Option<&str> {
        self.loaders
            .iter()
            .find_map(|l| matches!(l.load(), Ok(Some(_))).then(|| l.name()))
    }
}

impl TokenLoader for TokenChain {
    fn name(&self) -> &str {
        "chain"
    }

    fn load(&self) -> Result<Option<SecretString>> {
        for loader in &self.loaders {
            match loader.load() {
                Ok(Some(token)) => {
                    debug!(provider = loader.name(), "token loaded");
                    return Ok(Some(token));
                }
                Ok(None) => {
                    debug!(provider = loader.name(), "no token found");
                }
                Err(err) => {
                    warn!(provider = loader.name(), error = %err, "failed to load token, skipping");
                }
            }
        }

        Ok(None)
    }
}

impl TokenStore for TokenChain {
    fn name(&self) -> &str {
        "chain"
    }

    fn save(&self, token: &SecretString) -> Result<()> {
        for store in &self.stores {
            match store.save(token) {
                Ok(()) => {
                    debug!(provider = store.name(), "token saved");
                    return Ok(());
                }
                Err(err) => {
                    warn!(provider = store.name(), error = %err, "failed to save token, trying next");
                }
            }
        }

        bail!(t!("error-no-token-store"))
    }

    fn delete(&self) -> Result<()> {
        for store in &self.stores {
            match store.delete() {
                Ok(()) => debug!(provider = store.name(), "token deleted"),
                Err(err) => warn!(provider = store.name(), error = %err, "failed to delete token"),
            }
        }

        Ok(())
    }
}

const KEYRING_SERVICE: &str = "d1v-cli";
const KEYRING_USER: &str = "token";

impl Default for TokenChain {
    fn default() -> Self {
        Self::new(
            vec![
                Box::new(EnvProvider::new("D1V_AUTH_TOKEN")),
                Box::new(KeyringProvider::new(KEYRING_SERVICE, KEYRING_USER)),
                Box::new(ConfigProvider),
            ],
            vec![
                Box::new(KeyringProvider::new(KEYRING_SERVICE, KEYRING_USER)),
                Box::new(ConfigProvider),
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    struct InMemoryProvider {
        name: &'static str,
        token: Rc<RefCell<Option<SecretString>>>,
    }

    impl InMemoryProvider {
        fn new(name: &'static str) -> Self {
            Self {
                name,
                token: Rc::new(RefCell::new(None)),
            }
        }

        fn with_token(name: &'static str, token: &str) -> Self {
            Self {
                name,
                token: Rc::new(RefCell::new(Some(SecretString::from(token.to_string())))),
            }
        }

        fn pair(name: &'static str) -> (Self, Self) {
            let token = Rc::new(RefCell::new(None));
            (
                Self {
                    name,
                    token: token.clone(),
                },
                Self { name, token },
            )
        }
    }

    impl TokenLoader for InMemoryProvider {
        fn name(&self) -> &str {
            self.name
        }

        fn load(&self) -> Result<Option<SecretString>> {
            Ok(self.token.borrow().clone())
        }
    }

    impl TokenStore for InMemoryProvider {
        fn name(&self) -> &str {
            self.name
        }

        fn save(&self, token: &SecretString) -> Result<()> {
            *self.token.borrow_mut() = Some(token.clone());
            Ok(())
        }

        fn delete(&self) -> Result<()> {
            *self.token.borrow_mut() = None;
            Ok(())
        }
    }

    struct FailingLoader;

    impl TokenLoader for FailingLoader {
        fn name(&self) -> &str {
            "failing"
        }
        fn load(&self) -> Result<Option<SecretString>> {
            bail!("load failed")
        }
    }

    struct FailingStore;

    impl TokenStore for FailingStore {
        fn name(&self) -> &str {
            "failing"
        }
        fn save(&self, _: &SecretString) -> Result<()> {
            bail!("save failed")
        }
        fn delete(&self) -> Result<()> {
            bail!("delete failed")
        }
    }

    #[test]
    fn load_first_found() {
        let chain = TokenChain::new(
            vec![
                Box::new(InMemoryProvider::new("empty")),
                Box::new(InMemoryProvider::with_token("first", "secret-1")),
                Box::new(InMemoryProvider::with_token("second", "secret-2")),
            ],
            vec![],
        );
        let token = chain.load().unwrap().unwrap();
        assert_eq!(token.expose_secret(), "secret-1");
    }

    #[test]
    fn load_empty_chain() {
        let chain = TokenChain::new(vec![], vec![]);
        assert!(chain.load().unwrap().is_none());
    }

    #[test]
    fn load_skips_errors() {
        let chain = TokenChain::new(
            vec![
                Box::new(FailingLoader),
                Box::new(InMemoryProvider::with_token("fallback", "secret")),
            ],
            vec![],
        );
        let token = chain.load().unwrap().unwrap();
        assert_eq!(token.expose_secret(), "secret");
    }

    #[test]
    fn round_trip() {
        let (loader, store) = InMemoryProvider::pair("mem");
        let chain = TokenChain::new(vec![Box::new(loader)], vec![Box::new(store)]);

        assert!(chain.load().unwrap().is_none());

        let token = SecretString::from("round-trip-token");
        chain.save(&token).unwrap();

        let loaded = chain.load().unwrap().unwrap();
        assert_eq!(loaded.expose_secret(), "round-trip-token");
    }

    #[test]
    fn save_skips_errors() {
        let (loader, store) = InMemoryProvider::pair("fallback");
        let chain = TokenChain::new(
            vec![Box::new(loader)],
            vec![Box::new(FailingStore), Box::new(store)],
        );

        let token = SecretString::from("test-token");
        chain.save(&token).unwrap();

        let loaded = chain.load().unwrap().unwrap();
        assert_eq!(loaded.expose_secret(), "test-token");
    }

    #[test]
    fn save_no_stores() {
        let chain = TokenChain::new(vec![], vec![]);
        let token = SecretString::from("test-token");
        assert!(chain.save(&token).is_err());
    }

    #[test]
    fn delete_all_stores() {
        let (loader1, store1) = InMemoryProvider::pair("s1");
        let (loader2, store2) = InMemoryProvider::pair("s2");

        let chain = TokenChain::new(
            vec![Box::new(loader1), Box::new(loader2)],
            vec![Box::new(store1), Box::new(store2)],
        );

        chain.save(&SecretString::from("t1")).unwrap();
        chain.delete().unwrap();

        assert!(chain.load().unwrap().is_none());
    }
}
