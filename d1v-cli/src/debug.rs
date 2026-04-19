use d1v_api::jwt::Claims;
use d1v_api::UserAgent;
use owo_colors::{OwoColorize, Stream};
use serde::Serialize;
use std::fmt;
use std::fmt::{Display, Formatter, Write};

use crate::config::Config;
use crate::error::Result;
use crate::output::format_duration;
use crate::output::pad_label;
use crate::Context;
use crate::{i18n, symbols, t};

#[derive(Debug, Serialize)]
struct DebugInfo {
    version: String,
    user_agent: String,
    locale: String,
    features: String,
    config: String,
    log_dir: String,
    base_url: String,
    token: String,
}

impl Display for DebugInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let rows: &[(&str, &str)] = &[
            ("debug-label-version", &self.version),
            ("debug-label-user-agent", &self.user_agent),
            ("debug-label-locale", &self.locale),
            ("debug-label-features", &self.features),
            ("debug-label-config", &self.config),
            ("debug-label-log-dir", &self.log_dir),
            ("debug-label-base-url", &self.base_url),
            ("debug-label-token", &self.token),
        ];

        for (i, (key, value)) in rows.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(
                f,
                "{}{}",
                pad_label(t!(key), 13).if_supports_color(Stream::Stdout, |s| s.bold()),
                value.if_supports_color(Stream::Stdout, |s| s.cyan()),
            )?;
        }

        Ok(())
    }
}

fn write_claims(mut status: impl Write, claims: &Claims) -> fmt::Result {
    if let Some(subject) = &claims.subject
        && !subject.is_empty()
    {
        write!(status, " {subject}")?;
    }

    if let Some(duration) = claims.expires_in() {
        let formatted = format_duration(duration.as_secs());
        write!(
            status,
            " ({})",
            t!("debug-token-expires-in", duration = formatted)
        )?;
    } else if claims.is_expired() {
        write!(status, " ({})", t!("debug-token-expired"))?;
    }

    Ok(())
}

fn token_status(ctx: &Context) -> String {
    let Some(source) = ctx.tokens.source() else {
        return symbols::ERROR
            .if_supports_color(Stream::Stdout, |s| s.bright_red())
            .to_string();
    };

    let mut status = format!(
        "{} ({})",
        symbols::SUCCESS.if_supports_color(Stream::Stdout, |s| s.green()),
        t!("debug-token-found", source = source)
    );

    if let Some(claims) = ctx.client.claims() {
        write_claims(&mut status, &claims).unwrap();
    }

    status
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
        token: token_status(ctx),
    };

    ctx.print(&info)
}
