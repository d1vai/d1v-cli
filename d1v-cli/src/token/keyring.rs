use std::sync::LazyLock;

use secrecy::{ExposeSecret, SecretString};
use tracing::{debug, warn};

use super::{Result, TokenError, TokenLoader, TokenStore};

/// Stores token in the OS keychain.
pub struct KeyringProvider {
    service: &'static str,
    user: &'static str,
}

impl KeyringProvider {
    pub fn new(service: &'static str, user: &'static str) -> Self {
        Self { service, user }
    }

    fn entry(&self) -> Result<keyring_core::Entry> {
        ensure_keyring_store()?;
        keyring_core::Entry::new(self.service, self.user).map_err(|err| {
            debug!(error = %err, "failed to create keyring entry");
            TokenError::KeyringUnavailable
        })
    }
}

impl TokenLoader for KeyringProvider {
    fn name(&self) -> &'static str {
        "keyring"
    }

    fn load(&self) -> Result<Option<SecretString>> {
        let Ok(entry) = self.entry() else {
            return Ok(None);
        };

        match entry.get_password() {
            Ok(password) => Ok(Some(SecretString::from(password))),
            Err(keyring_core::Error::NoEntry) => Ok(None),
            Err(err) => {
                debug!(error = %err, "failed to load keyring credential");
                Ok(None)
            }
        }
    }
}

impl TokenStore for KeyringProvider {
    fn name(&self) -> &'static str {
        "keyring"
    }

    fn save(&self, token: &SecretString) -> Result {
        let entry = self.entry()?;
        entry
            .set_password(token.expose_secret())
            .map_err(TokenError::KeyringSave)
    }

    fn delete(&self) -> Result {
        let Ok(entry) = self.entry() else {
            return Ok(());
        };

        match entry.delete_credential() {
            Ok(()) => {}
            Err(keyring_core::Error::NoEntry) => {
                debug!("no keyring credential to delete");
            }
            Err(err) => {
                warn!(error = %err, "failed to delete keyring credential");
            }
        }

        Ok(())
    }
}

static KEYRING_STORE_AVAILABLE: LazyLock<bool> = LazyLock::new(|| {
    install_keyring_store()
        .inspect_err(|err| debug!(error = %err, "keyring store is not available"))
        .is_ok()
});

fn ensure_keyring_store() -> Result {
    (*KEYRING_STORE_AVAILABLE)
        .then_some(())
        .ok_or(TokenError::KeyringUnavailable)
}

fn install_keyring_store() -> std::result::Result<(), keyring_core::Error> {
    #[cfg(target_os = "linux")]
    {
        keyring_core::set_default_store(zbus_secret_service_keyring_store::Store::new()?);
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        keyring_core::set_default_store(apple_native_keyring_store::keychain::Store::new()?);
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        keyring_core::set_default_store(windows_native_keyring_store::Store::new()?);
        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Err(keyring_core::Error::NotSupportedByStore(
            "no keyring store configured for this platform".to_string(),
        ))
    }
}
