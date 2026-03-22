use anyhow::Result;
use d1v_api::UserAgent;
use serde::Serialize;
use std::fmt;
use std::fmt::{Display, Formatter};

use crate::config::Config;
use crate::t;
use crate::token::TokenChain;
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
        writeln!(f, "{:<13}{}", t!("debug-label-version"), self.version)?;
        writeln!(f, "{:<13}{}", t!("debug-label-user-agent"), self.user_agent)?;
        writeln!(f, "{:<13}{}", t!("debug-label-config"), self.config)?;
        writeln!(f, "{:<13}{}", t!("debug-label-base-url"), self.base_url)?;
        write!(f, "{:<13}{}", t!("debug-label-token"), self.token)
    }
}

pub fn run(ctx: &Context) -> Result<()> {
    let ua = UserAgent::new("d1v-cli", env!("CARGO_PKG_VERSION"));

    let config = Config::load()?;
    let config_path = Config::path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| t!("debug-unknown"));

    let token_status = match TokenChain::default().source() {
        Some(source) => t!("debug-token-found", source = source),
        None => t!("debug-token-missing"),
    };

    let info = DebugInfo {
        version: env!("CARGO_PKG_VERSION").into(),
        user_agent: ua.to_string(),
        config: config_path,
        base_url: config.base_url,
        token: token_status,
    };

    ctx.print(&info)
}
