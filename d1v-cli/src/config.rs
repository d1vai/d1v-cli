use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_base_url")]
    pub base_url: String,
    #[serde(
        serialize_with = "serialize_token",
        skip_serializing_if = "Option::is_none"
    )]
    pub token: Option<SecretString>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            base_url: default_base_url(),
            token: None,
        }
    }
}

fn default_base_url() -> String {
    d1v_api::DEFAULT_BASE_URL.to_string()
}

fn serialize_token<S>(token: &Option<SecretString>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match token {
        Some(token) => serializer.serialize_str(token.expose_secret()),
        None => serializer.serialize_none(),
    }
}

impl Config {
    pub fn dir() -> Result<PathBuf> {
        dirs::home_dir()
            .map(|p| p.join(".d1v"))
            .context("could not determine home directory")
    }

    pub fn path() -> Result<PathBuf> {
        Ok(Self::dir()?.join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::path()?;

        if !path.exists() {
            debug!(path = %path.display(), "config not found, using defaults");
            return Ok(Self::default());
        }

        debug!(path = %path.display(), "loading config");
        let content = fs::read_to_string(&path).context("failed to read config file")?;
        toml::from_str(&content).context("failed to parse config file")
    }

    pub fn save(&self) -> Result<()> {
        let dir = Self::dir()?;
        fs::create_dir_all(&dir)?;

        let path = dir.join("config.toml");
        let content = toml::to_string_pretty(self).context("failed to serialize config")?;
        fs::write(&path, &content)?;

        Ok(())
    }
}
