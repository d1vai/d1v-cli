use std::sync::{LazyLock, mpsc};
use std::time::Duration;

use secrecy::{ExposeSecret, SecretString};
use tracing::debug;

use super::{Result, TokenError, TokenSource, TokenStore};

pub const SERVICE: &str = "d1v-cli";
pub const USER: &str = "token";
const LOAD_TIMEOUT: Duration = Duration::from_secs(3);

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
        let service = self.service;
        let user = self.user;
        run_lookup_with_timeout(LOAD_TIMEOUT, move || load_password(service, user))
    }
}

fn run_lookup_with_timeout<F>(timeout: Duration, lookup: F) -> Result<Option<SecretString>>
where
    F: FnOnce() -> Result<Option<SecretString>> + Send + 'static,
{
    let (sender, receiver) = mpsc::sync_channel(1);
    std::thread::Builder::new()
        .name("d1v-keyring-load".into())
        .spawn(move || {
            let _ = sender.send(lookup());
        })
        .map_err(|_| TokenError::KeyringUnavailable)?;

    match receiver.recv_timeout(timeout) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => Err(TokenError::KeyringLoadTimeout),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(TokenError::KeyringUnavailable),
    }
}

fn load_password(service: &'static str, user: &'static str) -> Result<Option<SecretString>> {
    if !*KEYRING_STORE_AVAILABLE {
        return Err(TokenError::KeyringUnavailable);
    }

    let provider = KeyringProvider::new(service, user);
    let entry = match provider.entry() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyring_load_timeout_is_bounded() {
        let result = run_lookup_with_timeout(Duration::from_millis(5), || {
            std::thread::sleep(Duration::from_millis(50));
            Ok(None)
        });

        assert!(matches!(result, Err(TokenError::KeyringLoadTimeout)));
    }
}
