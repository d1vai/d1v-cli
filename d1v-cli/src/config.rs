use std::fs;
use std::path::PathBuf;

use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::debug;

type Result<T = (), E = ConfigError> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("could not determine home directory")]
    NoHomeDir,

    #[error("failed to read config file")]
    Read(#[source] std::io::Error),

    #[error("failed to write config file")]
    Write(#[source] std::io::Error),

    #[error("failed to parse config file")]
    Parse(#[from] toml::de::Error),

    #[error("failed to serialize config")]
    Serialize(#[from] toml::ser::Error),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_base_url")]
    pub base_url: String,

    #[serde(
        serialize_with = "serialize_token",
        skip_serializing_if = "Option::is_none"
    )]
    pub token: Option<SecretString>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,

    #[cfg(feature = "record")]
    #[serde(default, skip_serializing_if = "RecordConfig::is_default")]
    pub record: RecordConfig,
}

/// Configuration for HTTP recording.
///
/// ```toml
/// [record]
/// enabled = true
/// dir = "~/.d1v/recordings"
/// ```
#[cfg(feature = "record")]
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RecordConfig {
    /// Equivalent to passing `--record`.
    #[serde(default)]
    pub enabled: bool,

    /// Recording directory. Files are named `{date}.json`.
    /// Defaults to `~/.d1v/recordings/`.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_path"
    )]
    pub dir: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            base_url: default_base_url(),
            token: None,
            language: None,
            #[cfg(feature = "record")]
            record: RecordConfig::default(),
        }
    }
}

#[cfg(feature = "record")]
impl RecordConfig {
    fn is_default(&self) -> bool {
        !self.enabled && self.dir.is_none()
    }

    /// Resolves the recording file path, or `None` when disabled.
    pub fn resolve_path(&self) -> Option<PathBuf> {
        if self.enabled {
            record_path(self.dir.as_deref()).ok()
        } else {
            None
        }
    }
}

/// Returns a date-stamped recording path (`{dir}/{date}.json`),
/// falling back to `~/.d1v/recordings/` when `dir` is `None`.
#[cfg(feature = "record")]
pub fn record_path(dir: Option<&std::path::Path>) -> Result<PathBuf> {
    let dir = match dir {
        Some(d) => d.to_path_buf(),
        None => Config::dir()?.join("recordings"),
    };

    let today = jiff::Zoned::now().date();
    Ok(dir.join(format!("{today}.json")))
}

fn default_base_url() -> String {
    d1v_api::DEFAULT_BASE_URL.to_string()
}

/// Deserializes an optional path, expanding `~` to the home directory.
#[cfg(feature = "record")]
fn deserialize_path<'de, D>(deserializer: D) -> std::result::Result<Option<PathBuf>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<&str>::deserialize(deserializer)?
        .map(|s| PathBuf::from(shellexpand::tilde(s).as_ref())))
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
            .ok_or(ConfigError::NoHomeDir)
    }

    pub fn path() -> Result<PathBuf> {
        Ok(Self::dir()?.join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::path()?;

        if !path.exists() {
            debug!(path = %path.display(), "config not found, creating defaults");

            let config = Self::default();
            config.save()?;

            return Ok(config);
        }

        debug!(path = %path.display(), "loading config");
        let content = fs::read_to_string(&path).map_err(ConfigError::Read)?;
        toml::from_str(&content).map_err(ConfigError::Parse)
    }

    pub fn save(&self) -> Result {
        let dir = Self::dir()?;
        fs::create_dir_all(&dir).map_err(ConfigError::Write)?;

        let path = dir.join("config.toml");
        let content = toml::to_string_pretty(self)?;
        fs::write(&path, &content).map_err(ConfigError::Write)?;
        debug!(path = %path.display(), "config saved");

        Ok(())
    }
}
