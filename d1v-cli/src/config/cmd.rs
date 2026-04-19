use std::fmt::{self, Display, Formatter};

use clap::ValueEnum;
use owo_colors::{OwoColorize, Stream};
use serde::Serialize;
use tracing::debug;

use crate::config::{Config, ConfigError};
use crate::error::Result;
use crate::output::pad_label;
use crate::{t, Context};

#[derive(Debug, Clone, Copy, ValueEnum, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum ConfigKey {
    #[value(name = "base_url")]
    BaseUrl,
    #[value(name = "language")]
    Language,
    #[cfg(feature = "record")]
    #[value(name = "record.enabled")]
    #[strum(serialize = "record.enabled")]
    RecordEnabled,
    #[cfg(feature = "record")]
    #[value(name = "record.dir")]
    #[strum(serialize = "record.dir")]
    RecordDir,
}

impl Config {
    fn get(&self, key: ConfigKey) -> Option<String> {
        match key {
            ConfigKey::BaseUrl => Some(self.base_url.clone()),
            ConfigKey::Language => self.language.clone(),
            #[cfg(feature = "record")]
            ConfigKey::RecordEnabled => Some(self.record.enabled.to_string()),
            #[cfg(feature = "record")]
            ConfigKey::RecordDir => self.record.dir.as_ref().map(|p| p.display().to_string()),
        }
    }

    fn set(
        &mut self,
        key: ConfigKey,
        value: impl Into<String>,
    ) -> std::result::Result<(), ConfigError> {
        let value = value.into();

        match key {
            ConfigKey::BaseUrl => {
                self.base_url = if value.is_empty() {
                    d1v_api::DEFAULT_BASE_URL.to_string()
                } else {
                    value
                };
            }
            ConfigKey::Language => {
                self.language = if value.is_empty() { None } else { Some(value) };
            }
            #[cfg(feature = "record")]
            ConfigKey::RecordEnabled => {
                self.record.enabled =
                    value
                        .parse::<bool>()
                        .map_err(|_| ConfigError::InvalidValue {
                            key: key.to_string(),
                            value,
                        })?;
            }
            #[cfg(feature = "record")]
            ConfigKey::RecordDir => {
                self.record.dir = if value.is_empty() {
                    None
                } else {
                    Some(std::path::PathBuf::from(value))
                };
            }
        }
        Ok(())
    }
}

#[derive(Debug, Serialize)]
struct ConfigInfo {
    base_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    language: Option<String>,
    #[cfg(feature = "record")]
    record_enabled: bool,
    #[cfg(feature = "record")]
    #[serde(skip_serializing_if = "Option::is_none")]
    record_dir: Option<String>,
}

impl From<&Config> for ConfigInfo {
    fn from(config: &Config) -> Self {
        Self {
            base_url: config.base_url.clone(),
            language: config.language.clone(),
            #[cfg(feature = "record")]
            record_enabled: config.record.enabled,
            #[cfg(feature = "record")]
            record_dir: config.record.dir.as_ref().map(|p| p.display().to_string()),
        }
    }
}

impl Display for ConfigInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        const LABEL_WIDTH: usize = 16;

        let rows: Vec<(&str, String)> = vec![
            ("base_url", self.base_url.clone()),
            ("language", self.language.clone().unwrap_or_default()),
            #[cfg(feature = "record")]
            ("record.enabled", self.record_enabled.to_string()),
            #[cfg(feature = "record")]
            ("record.dir", self.record_dir.clone().unwrap_or_default()),
        ];

        for (i, (key, value)) in rows.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(
                f,
                "{}{}",
                pad_label(key, LABEL_WIDTH).if_supports_color(Stream::Stdout, |s| s.bold()),
                value.if_supports_color(Stream::Stdout, |s| s.cyan()),
            )?;
        }

        Ok(())
    }
}

pub fn show(ctx: &Context) -> Result<()> {
    let config = Config::load()?;
    ctx.print(&ConfigInfo::from(&config))
}

#[derive(Debug, Serialize)]
struct ConfigValue {
    key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,
}

impl Display for ConfigValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if let Some(ref v) = self.value {
            write!(f, "{v}")
        } else {
            Ok(())
        }
    }
}

pub fn get(ctx: &Context, key: ConfigKey) -> Result<()> {
    let config = Config::load()?;
    let value = config.get(key);

    ctx.print(&ConfigValue {
        key: key.to_string(),
        value,
    })
}

pub fn set(ctx: &Context, key: ConfigKey, value: &str) -> Result<()> {
    let mut config = Config::load()?;
    config.set(key, value)?;
    config.save()?;

    debug!(%key, value, "config key updated");
    ctx.success(t!("config-set-success", key = key, value = value));
    Ok(())
}

pub fn list(ctx: &Context) -> Result<()> {
    let keys: Vec<String> = ConfigKey::value_variants()
        .iter()
        .map(|k| k.to_string())
        .collect();

    ctx.print_list(keys)
}

#[derive(Debug, Serialize)]
struct ConfigPath {
    path: String,
}

impl Display for ConfigPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path)
    }
}

pub fn path(ctx: &Context) -> Result<()> {
    let path = Config::path()?;

    ctx.print(&ConfigPath {
        path: path.display().to_string(),
    })
}

pub fn reset(ctx: &Context) -> Result<()> {
    let config = Config::default();
    config.save()?;

    debug!("config reset to defaults");
    ctx.success(t!("config-reset-success"));
    Ok(())
}

pub fn edit() -> Result<()> {
    let path = Config::path()?;

    if !path.exists() {
        Config::default().save()?;
    }

    debug!(path = %path.display(), "opening config in editor");
    open::that(&path).map_err(ConfigError::Open)?;

    Ok(())
}
