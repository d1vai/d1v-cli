use secrecy::SecretString;
use std::sync::Mutex;
use tracing::{debug, warn};

use super::config::ConfigProvider;
use super::env::EnvProvider;
use super::keyring::{self, KeyringProvider};
use super::{Result, TokenError, TokenSource, TokenStore};

/// Chains multiple providers in priority order.
///
/// `lookup` returns the first token found;
/// `save` writes to the first store that succeeds;
/// `delete` removes from all stores.
pub struct TokenChain {
    sources: Vec<Box<dyn TokenSource>>,
    stores: Vec<Box<dyn TokenStore>>,
    cached_lookup: Mutex<Option<Option<CachedLookup>>>,
}

#[derive(Clone)]
struct CachedLookup {
    source: &'static str,
    token: SecretString,
}

impl TokenChain {
    pub fn new(sources: Vec<Box<dyn TokenSource>>, stores: Vec<Box<dyn TokenStore>>) -> Self {
        Self {
            sources,
            stores,
            cached_lookup: Mutex::new(None),
        }
    }

    /// Returns the name of the first source that provides a token.
    pub fn source(&self) -> Option<&str> {
        self.lookup_with_source().map(|(source, _)| source)
    }

    fn lookup_with_source(&self) -> Option<(&'static str, SecretString)> {
        if let Some(cached) = self.cached_lookup.lock().unwrap().as_ref() {
            return cached
                .as_ref()
                .map(|value| (value.source, value.token.clone()));
        }

        let mut found = None;
        for source in &self.sources {
            match source.lookup() {
                Ok(Some(token)) => {
                    debug!(provider = source.name(), "token loaded");
                    found = Some(CachedLookup {
                        source: source.name(),
                        token,
                    });
                    break;
                }
                Ok(None) => {
                    debug!(provider = source.name(), "no token found");
                }
                Err(err) => {
                    debug!(provider = source.name(), error = %err, "failed to load token, skipping");
                }
            }
        }

        *self.cached_lookup.lock().unwrap() = Some(found.clone());
        found.map(|value| (value.source, value.token))
    }
}

impl TokenSource for TokenChain {
    fn name(&self) -> &'static str {
        "chain"
    }

    fn lookup(&self) -> Result<Option<SecretString>> {
        Ok(self.lookup_with_source().map(|(_, token)| token))
    }
}

impl TokenStore for TokenChain {
    fn name(&self) -> &'static str {
        "chain"
    }

    fn save(&self, token: &SecretString) -> Result {
        for store in &self.stores {
            match store.save(token) {
                Ok(()) => {
                    debug!(provider = store.name(), "token saved");
                    *self.cached_lookup.lock().unwrap() = Some(Some(CachedLookup {
                        source: store.name(),
                        token: token.clone(),
                    }));
                    return Ok(());
                }
                Err(err) => {
                    warn!(provider = store.name(), error = %err, "failed to save token, trying next");
                }
            }
        }

        Err(TokenError::NoStore)
    }

    fn delete(&self) -> Result {
        for store in &self.stores {
            match store.delete() {
                Ok(()) => debug!(provider = store.name(), "token deleted"),
                Err(err) => warn!(provider = store.name(), error = %err, "failed to delete token"),
            }
        }

        *self.cached_lookup.lock().unwrap() = Some(None);

        Ok(())
    }
}

impl Default for TokenChain {
    fn default() -> Self {
        Self::new(
            vec![
                Box::new(EnvProvider::new("D1V_API_KEY")),
                Box::new(EnvProvider::new("D1V_AUTH_TOKEN")),
                Box::new(KeyringProvider::new(keyring::SERVICE, keyring::USER)),
                Box::new(ConfigProvider),
            ],
            vec![
                Box::new(KeyringProvider::new(keyring::SERVICE, keyring::USER)),
                Box::new(ConfigProvider),
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::ExposeSecret;
    use std::cell::{Cell, RefCell};
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

    impl TokenSource for InMemoryProvider {
        fn name(&self) -> &'static str {
            self.name
        }

        fn lookup(&self) -> Result<Option<SecretString>> {
            Ok(self.token.borrow().clone())
        }
    }

    impl TokenStore for InMemoryProvider {
        fn name(&self) -> &'static str {
            self.name
        }

        fn save(&self, token: &SecretString) -> Result {
            *self.token.borrow_mut() = Some(token.clone());
            Ok(())
        }

        fn delete(&self) -> Result {
            *self.token.borrow_mut() = None;
            Ok(())
        }
    }

    struct FailingLoader;

    impl TokenSource for FailingLoader {
        fn name(&self) -> &'static str {
            "failing"
        }

        fn lookup(&self) -> Result<Option<SecretString>> {
            Err(TokenError::KeyringUnavailable)
        }
    }

    struct FailingStore;

    struct CountingProvider {
        calls: Rc<Cell<usize>>,
    }

    impl TokenSource for CountingProvider {
        fn name(&self) -> &'static str {
            "counting"
        }

        fn lookup(&self) -> Result<Option<SecretString>> {
            self.calls.set(self.calls.get() + 1);
            Ok(Some(SecretString::from("secret".to_string())))
        }
    }

    impl TokenStore for FailingStore {
        fn name(&self) -> &'static str {
            "failing"
        }

        fn save(&self, _: &SecretString) -> Result {
            Err(TokenError::KeyringUnavailable)
        }

        fn delete(&self) -> Result {
            Err(TokenError::KeyringUnavailable)
        }
    }

    #[test]
    fn lookup_first_found() {
        let chain = TokenChain::new(
            vec![
                Box::new(InMemoryProvider::new("empty")),
                Box::new(InMemoryProvider::with_token("first", "secret-1")),
                Box::new(InMemoryProvider::with_token("second", "secret-2")),
            ],
            vec![],
        );
        let token = chain.lookup().unwrap().unwrap();
        assert_eq!(token.expose_secret(), "secret-1");
    }

    #[test]
    fn lookup_empty_chain() {
        let chain = TokenChain::new(vec![], vec![]);
        assert!(chain.lookup().unwrap().is_none());
    }

    #[test]
    fn lookup_skips_errors() {
        let chain = TokenChain::new(
            vec![
                Box::new(FailingLoader),
                Box::new(InMemoryProvider::with_token("fallback", "secret")),
            ],
            vec![],
        );
        let token = chain.lookup().unwrap().unwrap();
        assert_eq!(token.expose_secret(), "secret");
    }

    #[test]
    fn lookup_and_source_reuse_cached_provider_result() {
        let calls = Rc::new(Cell::new(0));
        let chain = TokenChain::new(
            vec![Box::new(CountingProvider {
                calls: calls.clone(),
            })],
            vec![],
        );

        assert_eq!(chain.lookup().unwrap().unwrap().expose_secret(), "secret");
        assert_eq!(chain.source(), Some("counting"));
        assert_eq!(chain.lookup().unwrap().unwrap().expose_secret(), "secret");
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn round_trip() {
        let (source, store) = InMemoryProvider::pair("mem");
        let chain = TokenChain::new(vec![Box::new(source)], vec![Box::new(store)]);

        assert!(chain.lookup().unwrap().is_none());

        let token = SecretString::from("round-trip-token");
        chain.save(&token).unwrap();

        let loaded = chain.lookup().unwrap().unwrap();
        assert_eq!(loaded.expose_secret(), "round-trip-token");
    }

    #[test]
    fn save_skips_errors() {
        let (source, store) = InMemoryProvider::pair("fallback");
        let chain = TokenChain::new(
            vec![Box::new(source)],
            vec![Box::new(FailingStore), Box::new(store)],
        );

        let token = SecretString::from("test-token");
        chain.save(&token).unwrap();

        let loaded = chain.lookup().unwrap().unwrap();
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
        let (source1, store1) = InMemoryProvider::pair("s1");
        let (source2, store2) = InMemoryProvider::pair("s2");

        let chain = TokenChain::new(
            vec![Box::new(source1), Box::new(source2)],
            vec![Box::new(store1), Box::new(store2)],
        );

        chain.save(&SecretString::from("t1")).unwrap();
        chain.delete().unwrap();

        assert!(chain.lookup().unwrap().is_none());
    }
}
