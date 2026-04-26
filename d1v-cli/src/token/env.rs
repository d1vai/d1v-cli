use secrecy::SecretString;

use super::{Result, TokenSource};

/// Reads token from an environment variable.
pub struct EnvProvider {
    var_name: &'static str,
}

impl EnvProvider {
    pub fn new(var_name: &'static str) -> Self {
        Self { var_name }
    }
}

impl TokenSource for EnvProvider {
    fn name(&self) -> &'static str {
        self.var_name
    }

    fn lookup(&self) -> Result<Option<SecretString>> {
        match std::env::var(self.var_name) {
            Ok(v) if !v.is_empty() => Ok(Some(SecretString::from(v))),
            _ => Ok(None),
        }
    }
}
