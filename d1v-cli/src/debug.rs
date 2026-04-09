use d1v_api::UserAgent;
use serde::Serialize;
use std::fmt;
use std::fmt::{Display, Formatter};

use crate::config::Config;
use crate::error::Result;
use crate::output::pad_label;
use crate::t;
use crate::Context;

#[derive(Debug, Serialize)]
struct DebugInfo {
    version: String,
    user_agent: String,
    config: String,
    base_url: String,
    token: String,
}

impl Display for DebugInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{}{}",
            pad_label(&t!("debug-label-version"), 13),
            self.version
        )?;
        writeln!(
            f,
            "{}{}",
            pad_label(&t!("debug-label-user-agent"), 13),
            self.user_agent
        )?;
        writeln!(
            f,
            "{}{}",
            pad_label(&t!("debug-label-config"), 13),
            self.config
        )?;
        writeln!(
            f,
            "{}{}",
            pad_label(&t!("debug-label-base-url"), 13),
            self.base_url
        )?;
        write!(
            f,
            "{}{}",
            pad_label(&t!("debug-label-token"), 13),
            self.token
        )
    }
}

fn token_status(ctx: &Context) -> String {
    let Some(source) = ctx.tokens.source() else {
        return t!("debug-token-missing");
    };

    let base = t!("debug-token-found", source = source);

    let Some(Ok(claims)) = ctx.client.claims() else {
        return base;
    };

    let claims = claims.to_string();
    if claims.is_empty() {
        return base;
    }

    format!("{base} {claims}")
}

pub fn run(ctx: &Context) -> Result<()> {
    let ua = UserAgent::new("d1v-cli", env!("CARGO_PKG_VERSION"));

    let config_path = Config::path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| t!("debug-unknown"));

    let info = DebugInfo {
        version: env!("CARGO_PKG_VERSION").into(),
        user_agent: ua.to_string(),
        config: config_path,
        base_url: ctx.client.base_url().to_string(),
        token: token_status(ctx),
    };

    ctx.print(&info)
}
