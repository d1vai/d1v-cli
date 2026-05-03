use clap::ValueEnum;
use serde::Serialize;
use tracing::debug;

use crate::config::{Config, ConfigError};
use crate::error::Result;
use crate::text::{Field, Fields, Render, RenderContext, Span, Text};
use crate::{Context, t, theme};

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

struct ConfigInfoView<'a> {
    info: &'a ConfigInfo,
}

impl Render for ConfigInfoView<'_> {
    fn render(&self, ctx: &mut RenderContext<'_>) -> std::io::Result<()> {
        let fields = vec![
            Field::new(
                Span::styled("base_url", theme::ansi::label()),
                Span::styled(self.info.base_url.clone(), theme::ansi::value()),
            ),
            Field::new(
                Span::styled("language", theme::ansi::label()),
                Span::styled(
                    self.info.language.clone().unwrap_or_default(),
                    theme::ansi::value(),
                ),
            ),
            #[cfg(feature = "record")]
            Field::new(
                Span::styled("record.enabled", theme::ansi::label()),
                Span::styled(self.info.record_enabled.to_string(), theme::ansi::value()),
            ),
            #[cfg(feature = "record")]
            Field::new(
                Span::styled("record.dir", theme::ansi::label()),
                Span::styled(
                    self.info.record_dir.clone().unwrap_or_default(),
                    theme::ansi::value(),
                ),
            ),
        ];

        Fields::new(fields).render(ctx)
    }
}

pub fn show(ctx: &Context) -> Result<()> {
    let config = Config::load()?;
    let info = ConfigInfo::from(&config);

    ctx.present(ConfigInfoView { info: &info }, &info)
}

#[derive(Debug, Serialize)]
struct ConfigValue {
    key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,
}

struct ConfigValueView<'a> {
    value: &'a ConfigValue,
}

impl Render for ConfigValueView<'_> {
    fn render(&self, ctx: &mut RenderContext<'_>) -> std::io::Result<()> {
        Text::new()
            .line(self.value.value.clone().unwrap_or_default())
            .render(ctx)
    }
}

pub fn get(ctx: &Context, key: ConfigKey) -> Result<()> {
    let config = Config::load()?;
    let value = ConfigValue {
        key: key.to_string(),
        value: config.get(key),
    };

    ctx.present(ConfigValueView { value: &value }, &value)
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
        .map(ToString::to_string)
        .collect();

    ctx.present(ConfigKeysView { keys: &keys }, &keys)
}

struct ConfigKeysView<'a> {
    keys: &'a [String],
}

impl Render for ConfigKeysView<'_> {
    fn render(&self, ctx: &mut RenderContext<'_>) -> std::io::Result<()> {
        Text::from_iter(self.keys.iter().cloned()).render(ctx)
    }
}

#[derive(Debug, Serialize)]
struct ConfigPath {
    path: String,
}

struct ConfigPathView<'a> {
    path: &'a ConfigPath,
}

impl Render for ConfigPathView<'_> {
    fn render(&self, ctx: &mut RenderContext<'_>) -> std::io::Result<()> {
        Text::new().line(self.path.path.clone()).render(ctx)
    }
}

pub fn path(ctx: &Context) -> Result<()> {
    let path = Config::path()?;
    let path = ConfigPath {
        path: path.display().to_string(),
    };

    ctx.present(ConfigPathView { path: &path }, &path)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::RenderExt;

    #[test]
    fn key_display() {
        assert_eq!(ConfigKey::BaseUrl.to_string(), "base_url");
        assert_eq!(ConfigKey::Language.to_string(), "language");
    }

    #[test]
    fn get_base_url() {
        let config = Config::default();
        assert!(config.get(ConfigKey::BaseUrl).is_some());
    }

    #[test]
    fn get_language_none() {
        let config = Config::default();
        assert!(config.get(ConfigKey::Language).is_none());
    }

    #[test]
    fn set_base_url() {
        let mut config = Config::default();
        config
            .set(ConfigKey::BaseUrl, "https://example.com")
            .unwrap();
        assert_eq!(config.base_url, "https://example.com");
    }

    #[test]
    fn set_base_url_empty() {
        let mut config = Config::default().base_url("https://example.com");
        config.set(ConfigKey::BaseUrl, "").unwrap();
        assert_eq!(config.base_url, d1v_api::DEFAULT_BASE_URL);
    }

    #[test]
    fn set_language() {
        let mut config = Config::default();
        config.set(ConfigKey::Language, "zh-Hans").unwrap();
        assert_eq!(config.language.as_deref(), Some("zh-Hans"));
    }

    #[test]
    fn set_language_empty() {
        let mut config = Config::default().language("en");
        config.set(ConfigKey::Language, "").unwrap();
        assert!(config.language.is_none());
    }

    #[test]
    fn info_text() {
        let config = Config {
            base_url: "https://api.d1v.ai".into(),
            language: Some("en".into()),
            ..Config::default()
        };

        let info = ConfigInfo::from(&config);
        let text = ConfigInfoView { info: &info }.display().to_string();

        assert!(text.contains("base_url"));
        assert!(text.contains("https://api.d1v.ai"));
        assert!(text.contains("language"));
        assert!(text.contains("en"));
    }

    #[test]
    fn info_json() {
        let config = Config {
            base_url: "https://api.d1v.ai".into(),
            language: Some("en".into()),
            ..Config::default()
        };

        let info = ConfigInfo::from(&config);
        let json: serde_json::Value = serde_json::to_value(&info).unwrap();

        assert_eq!(json["base_url"], "https://api.d1v.ai");
        assert_eq!(json["language"], "en");
    }

    #[test]
    fn value_text() {
        let with = ConfigValue {
            key: "base_url".into(),
            value: Some("https://api.d1v.ai".into()),
        };
        assert_eq!(
            ConfigValueView { value: &with }.display().to_string(),
            "https://api.d1v.ai\n"
        );

        let without = ConfigValue {
            key: "language".into(),
            value: None,
        };
        assert_eq!(
            ConfigValueView { value: &without }.display().to_string(),
            "\n"
        );
    }

    #[test]
    fn path_text() {
        let p = ConfigPath {
            path: "/home/user/.d1v/config.toml".into(),
        };
        assert_eq!(
            ConfigPathView { path: &p }.display().to_string(),
            "/home/user/.d1v/config.toml\n"
        );
    }

    #[test]
    fn keys_text() {
        let keys: Vec<String> = ConfigKey::value_variants()
            .iter()
            .map(ToString::to_string)
            .collect();

        let expected = [
            "base_url",
            "language",
            #[cfg(feature = "record")]
            "record.enabled",
            #[cfg(feature = "record")]
            "record.dir",
        ]
        .join("\n")
            + "\n";

        assert_eq!(
            ConfigKeysView { keys: &keys }.display().to_string(),
            expected
        );
    }
}
