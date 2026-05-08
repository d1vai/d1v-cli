pub mod cmd;

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

    #[error("invalid value for {key}: {value}")]
    InvalidValue { key: String, value: String },

    #[error("failed to open config file: {0}")]
    Open(#[source] std::io::Error),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_base_url"
    )]
    base_url: Option<String>,

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
            base_url: None,
            token: None,
            language: None,
            #[cfg(feature = "record")]
            record: RecordConfig::default(),
        }
    }
}

impl Config {
    /// Returns the effective base URL.
    pub fn base_url(&self) -> &str {
        self.base_url
            .as_deref()
            .unwrap_or(d1v_api::DEFAULT_BASE_URL)
    }

    /// Returns the user-configured base URL.
    pub fn base_url_override(&self) -> Option<&str> {
        self.base_url.as_deref()
    }

    /// Sets the base URL. Empty or whitespace-only input clears the override.
    pub fn set_base_url(&mut self, base_url: impl Into<String>) {
        let base_url = base_url.into();
        let trimmed = base_url.trim();

        self.base_url = if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        };
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

/// Treats empty or whitespace-only `base_url` as unset.
fn deserialize_base_url<'de, D>(deserializer: D) -> std::result::Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(deserializer)?
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty()))
}

/// Deserializes an optional path, expanding `~` and normalizing separators.
#[cfg(feature = "record")]
fn deserialize_path<'de, D>(deserializer: D) -> std::result::Result<Option<PathBuf>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use normpath::PathExt;

    Ok(Option::<&str>::deserialize(deserializer)?.map(|s| {
        let path = PathBuf::from(shellexpand::tilde(s).as_ref());
        path.normalize_virtually().map(Into::into).unwrap_or(path)
    }))
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

#[derive(Debug, Default)]
pub struct ConfigBuilder {
    inner: Config,
}

impl ConfigBuilder {
    #[must_use]
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.inner.set_base_url(base_url);
        self
    }

    #[must_use]
    pub fn token(mut self, token: SecretString) -> Self {
        self.inner.token = Some(token);
        self
    }

    #[must_use]
    pub fn language(mut self, language: impl Into<String>) -> Self {
        self.inner.language = Some(language.into());
        self
    }

    pub fn build(self) -> Config {
        self.inner
    }
}

impl Config {
    pub fn new() -> Config {
        Self::default()
    }

    #[must_use]
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_falls_back_to_default() {
        let config: Config = toml::from_str(r#"base_url = """#).unwrap();
        assert_eq!(config.base_url(), d1v_api::DEFAULT_BASE_URL);
        assert_eq!(config.base_url_override(), None);
    }

    #[test]
    fn whitespace_falls_back_to_default() {
        let config: Config = toml::from_str(r#"base_url = "   ""#).unwrap();
        assert_eq!(config.base_url(), d1v_api::DEFAULT_BASE_URL);
        assert_eq!(config.base_url_override(), None);
    }

    #[test]
    fn missing_uses_default() {
        let config: Config = toml::from_str("").unwrap();
        assert_eq!(config.base_url(), d1v_api::DEFAULT_BASE_URL);
        assert_eq!(config.base_url_override(), None);
    }

    #[test]
    fn trims_whitespace() {
        let config: Config = toml::from_str(r#"base_url = "  https://api.example.com  ""#).unwrap();
        assert_eq!(config.base_url(), "https://api.example.com");
        assert_eq!(config.base_url_override(), Some("https://api.example.com"));
    }
}
