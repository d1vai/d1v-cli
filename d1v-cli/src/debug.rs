use d1v_api::jwt::Claims;
use d1v_api::UserAgent;
use serde::Serialize;

use crate::config::Config;
use crate::error::Result;
use crate::output::format_duration;
use crate::text::{Field, Fields, Line, Render, RenderContext, Span};
use crate::Context;
use crate::{i18n, symbols, t, theme};

#[derive(Debug, Serialize)]
struct DebugInfo {
    version: String,
    user_agent: String,
    locale: String,
    features: String,
    config: String,
    log_dir: String,
    base_url: String,
    token: TokenInfo,
}

#[derive(Debug, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum TokenInfo {
    Missing,
    Found {
        source: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        subject: Option<String>,
        #[serde(flatten)]
        expiry: TokenExpiry,
    },
}

impl TokenInfo {
    fn from_context(ctx: &Context) -> Self {
        let Some(source) = ctx.tokens.source() else {
            return Self::Missing;
        };

        let Some(claims) = ctx.client.claims() else {
            return Self::Found {
                source: source.to_string(),
                subject: None,
                expiry: TokenExpiry::Unknown,
            };
        };

        Self::Found {
            source: source.to_string(),
            subject: claims.subject.clone().filter(|subject| !subject.is_empty()),
            expiry: TokenExpiry::from_claims(&claims),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "expiry", rename_all = "snake_case")]
enum TokenExpiry {
    Unknown,
    Expired,
    ExpiresIn { seconds: i64 },
}

impl TokenExpiry {
    fn from_claims(claims: &Claims) -> Self {
        if let Some(duration) = claims.expires_in() {
            Self::ExpiresIn {
                seconds: duration.as_secs(),
            }
        } else if claims.is_expired() {
            Self::Expired
        } else {
            Self::Unknown
        }
    }
}

struct DebugInfoView<'a> {
    info: &'a DebugInfo,
}

impl Render for DebugInfoView<'_> {
    fn render(&self, ctx: &mut RenderContext<'_>) -> std::io::Result<()> {
        let rows = [
            self.field("debug-label-version", self.info.version.as_str()),
            self.field("debug-label-user-agent", self.info.user_agent.as_str()),
            self.field("debug-label-locale", self.info.locale.as_str()),
            self.field("debug-label-features", self.info.features.as_str()),
            self.field("debug-label-config", self.info.config.as_str()),
            self.field("debug-label-log-dir", self.info.log_dir.as_str()),
            self.field("debug-label-base-url", self.info.base_url.as_str()),
            Field::new(
                Span::styled(t!("debug-label-token"), theme::ansi::label()),
                self.token_line(),
            ),
        ];

        Fields::new(rows).render(ctx)
    }
}

impl DebugInfoView<'_> {
    fn field(&self, key: &'static str, value: &str) -> Field {
        Field::new(
            Span::styled(t!(key), theme::ansi::label()),
            Line::styled(value.to_owned(), theme::ansi::value()),
        )
    }

    fn token_line(&self) -> Line {
        let TokenInfo::Found {
            source,
            subject,
            expiry,
        } = &self.info.token
        else {
            return Line::styled(symbols::ERROR, theme::ansi::error());
        };

        let mut line = Line::styled(symbols::SUCCESS, theme::ansi::success())
            .push_plain(" ")
            .push_styled(
                format!("({})", t!("debug-token-found", source = source)),
                theme::ansi::value(),
            );

        if let Some(subject) = subject {
            line = line
                .push_plain(" ")
                .push_styled(subject.clone(), theme::ansi::value());
        }

        match expiry {
            TokenExpiry::Unknown => line,
            TokenExpiry::Expired => line.push_plain(" ").push_styled(
                format!("({})", t!("debug-token-expired")),
                theme::ansi::value(),
            ),
            TokenExpiry::ExpiresIn { seconds } => line.push_plain(" ").push_styled(
                format!(
                    "({})",
                    t!(
                        "debug-token-expires-in",
                        duration = format_duration(*seconds)
                    )
                ),
                theme::ansi::value(),
            ),
        }
    }
}

fn enabled_features() -> String {
    let features: &[&str] = &[
        #[cfg(feature = "record")]
        "record",
        #[cfg(feature = "mock")]
        "mock",
    ];

    if features.is_empty() {
        t!("debug-features-none")
    } else {
        features.join(", ")
    }
}

pub fn run(ctx: &Context) -> Result<()> {
    let ua = UserAgent::new("d1v-cli", env!("CARGO_PKG_VERSION"));

    let config_path =
        Config::path().map_or_else(|_| t!("debug-unknown"), |p| p.display().to_string());

    let log_dir = Config::dir().map_or_else(|_| t!("debug-unknown"), |p| p.display().to_string());

    let info = DebugInfo {
        version: env!("CARGO_PKG_VERSION").into(),
        user_agent: ua.to_string(),
        locale: i18n::locale().to_string(),
        features: enabled_features(),
        config: config_path,
        log_dir,
        base_url: ctx.client.base_url().to_string(),
        token: TokenInfo::from_context(ctx),
    };

    ctx.present(DebugInfoView { info: &info }, &info)
}
