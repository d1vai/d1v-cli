//! Resolve the API base URL from CLI, env, config, or default.
//!
//! Empty values are skipped, so clearing `D1V_BASE_URL` falls through cleanly.

use itertools::Itertools;
use serde::Serialize;
use std::fmt::{self, Display};

/// Source of a resolved [`BaseUrl`].
///
/// Variant order is the resolution priority.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BaseUrlSource {
    /// `--base-url` flag.
    Cli,
    /// `D1V_BASE_URL` environment variable.
    Env,
    /// `base_url` from `~/.d1v/config.toml`.
    Config,
    /// Built-in [`d1v_api::DEFAULT_BASE_URL`].
    #[default]
    Default,
}

impl Display for BaseUrlSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Cli => "--base-url",
            Self::Env => "D1V_BASE_URL",
            Self::Config => "config file",
            Self::Default => "default",
        })
    }
}

/// One normalized optional layer in the base URL provider chain.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BaseUrlCandidate {
    source: BaseUrlSource,
    value: Option<String>,
}

impl BaseUrlCandidate {
    pub fn new(source: BaseUrlSource, value: Option<String>) -> Self {
        Self {
            source,
            value: value
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
        }
    }
}

pub fn from_cli(value: Option<String>) -> BaseUrlCandidate {
    BaseUrlCandidate::new(BaseUrlSource::Cli, value)
}

pub fn from_env(value: Option<String>) -> BaseUrlCandidate {
    BaseUrlCandidate::new(BaseUrlSource::Env, value)
}

pub fn from_config(value: Option<String>) -> BaseUrlCandidate {
    BaseUrlCandidate::new(BaseUrlSource::Config, value)
}

pub fn default() -> BaseUrlCandidate {
    BaseUrlCandidate::default()
}

/// Resolved base URL with its source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BaseUrl {
    url: String,
    source: BaseUrlSource,
}

impl BaseUrl {
    pub fn resolve(candidates: impl IntoIterator<Item = BaseUrlCandidate>) -> Self {
        candidates
            .into_iter()
            .sorted_by_key(|c| c.source)
            .find_map(|BaseUrlCandidate { source, value }| value.map(|url| BaseUrl { url, source }))
            .unwrap_or_else(|| BaseUrl {
                url: d1v_api::DEFAULT_BASE_URL.to_string(),
                source: BaseUrlSource::Default,
            })
    }

    pub fn as_str(&self) -> &str {
        &self.url
    }

    pub fn source(&self) -> BaseUrlSource {
        self.source
    }
}

impl Display for BaseUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_wins() {
        let resolved = BaseUrl::resolve([
            from_cli(Some("https://cli.example.com".into())),
            from_config(Some("https://cfg.example.com".into())),
        ]);
        assert_eq!(resolved.as_str(), "https://cli.example.com");
        assert_eq!(resolved.source(), BaseUrlSource::Cli);
    }

    #[test]
    fn env_used_when_no_cli() {
        let resolved = BaseUrl::resolve([from_env(Some("https://env.example.com".into()))]);
        assert_eq!(resolved.as_str(), "https://env.example.com");
        assert_eq!(resolved.source(), BaseUrlSource::Env);
    }

    #[test]
    fn empty_env_falls_through() {
        let resolved = BaseUrl::resolve([
            from_env(Some(String::new())),
            from_config(Some("https://cfg.example.com".into())),
        ]);
        assert_eq!(resolved.as_str(), "https://cfg.example.com");
        assert_eq!(resolved.source(), BaseUrlSource::Config);
    }

    #[test]
    fn whitespace_normalized() {
        let resolved = BaseUrl::resolve([from_cli(Some("  https://x  ".into()))]);
        assert_eq!(resolved.as_str(), "https://x");
        assert_eq!(resolved.source(), BaseUrlSource::Cli);
    }

    #[test]
    fn default_when_empty() {
        let resolved = BaseUrl::resolve([]);
        assert_eq!(resolved.as_str(), d1v_api::DEFAULT_BASE_URL);
        assert_eq!(resolved.source(), BaseUrlSource::Default);
    }

    #[test]
    fn display_writes_url() {
        let resolved = BaseUrl::resolve([from_cli(Some("https://x.example".into()))]);
        assert_eq!(resolved.to_string(), "https://x.example");
    }

    #[test]
    fn source_serializes_snake_case() {
        let json = serde_json::to_string(&BaseUrlSource::Cli).unwrap();
        assert_eq!(json, r#""cli""#);
        let json = serde_json::to_string(&BaseUrlSource::Default).unwrap();
        assert_eq!(json, r#""default""#);
    }

    /// Guards the priority encoded by [`BaseUrlSource`] order.
    #[test]
    fn priority_order() {
        assert!(BaseUrlSource::Cli < BaseUrlSource::Env);
        assert!(BaseUrlSource::Env < BaseUrlSource::Config);
        assert!(BaseUrlSource::Config < BaseUrlSource::Default);
    }
}
