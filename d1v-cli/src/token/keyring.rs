use std::sync::LazyLock;

use secrecy::{ExposeSecret, SecretString};
use tracing::debug;

use super::{Result, TokenError, TokenSource, TokenStore};

pub const SERVICE: &str = "d1v-cli";
pub const USER: &str = "token";

/// Stores token in the OS keychain.
pub struct KeyringProvider {
    service: &'static str,
    user: &'static str,
}

impl KeyringProvider {
    pub fn new(service: &'static str, user: &'static str) -> Self {
        Self { service, user }
    }

    fn entry(&self) -> Result<keyring_core::Entry, keyring_core::Error> {
        keyring_core::Entry::new(self.service, self.user).inspect_err(|err| {
            debug!(error = %err, "failed to create keyring entry");
        })
    }
}

impl TokenSource for KeyringProvider {
    fn name(&self) -> &'static str {
        "keyring"
    }

    fn lookup(&self) -> Result<Option<SecretString>> {
        if !*KEYRING_STORE_AVAILABLE {
            return Err(TokenError::KeyringUnavailable);
        }

        let entry = match self.entry() {
            Ok(entry) => entry,
            Err(err) => return Err(TokenError::KeyringLoad(err)),
        };

        match entry.get_password() {
            Ok(password) => Ok(Some(SecretString::from(password))),
            Err(keyring_core::Error::NoEntry) => Ok(None),
            Err(err) => {
                debug!(error = %err, "failed to load keyring credential");
                Err(TokenError::KeyringLoad(err))
            }
        }
    }
}

impl TokenStore for KeyringProvider {
    fn name(&self) -> &'static str {
        "keyring"
    }

    fn save(&self, token: &SecretString) -> Result {
        if !*KEYRING_STORE_AVAILABLE {
            return Err(TokenError::KeyringUnavailable);
        }

        let entry = self.entry().map_err(TokenError::KeyringSave)?;
        entry
            .set_password(token.expose_secret())
            .map_err(TokenError::KeyringSave)
    }

    fn delete(&self) -> Result {
        if !*KEYRING_STORE_AVAILABLE {
            return Ok(());
        }

        match self
            .entry()
            .map_err(TokenError::KeyringDelete)?
            .delete_credential()
        {
            Ok(()) => {}
            Err(keyring_core::Error::NoEntry) => {
                debug!("no keyring credential to delete");
            }
            Err(err) => {
                debug!(error = %err, "failed to delete keyring credential");
                return Err(TokenError::KeyringDelete(err));
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
