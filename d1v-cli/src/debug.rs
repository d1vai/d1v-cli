use anyhow::Result;
use d1v_api::UserAgent;
use serde::Serialize;
use std::fmt;
use std::fmt::{Display, Formatter};

use crate::config::Config;
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
        writeln!(f, "version:     {}", self.version)?;
        writeln!(f, "user-agent:  {}", self.user_agent)?;
        writeln!(f, "config:      {}", self.config)?;
        writeln!(f, "base-url:    {}", self.base_url)?;
        write!(f, "token:       {}", self.token)
    }
}

pub fn run(ctx: &Context) -> Result<()> {
    let ua = UserAgent::new("d1v-cli", env!("CARGO_PKG_VERSION"));

    let config = Config::load()?;
    let config_path = Config::path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "unknown".into());

    let token_status = match TokenChain::default().source() {
        Some(source) => format!("✓ ({source})"),
        None => "✗".into(),
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
