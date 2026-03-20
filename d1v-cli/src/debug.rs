use anyhow::Result;
use d1v_api::UserAgent;

use crate::config::Config;
use crate::token::TokenChain;

pub fn run() -> Result<()> {
    let ua = UserAgent::new("d1v-cli", env!("CARGO_PKG_VERSION"));

    let config = Config::load()?;
    let config_path = Config::path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "unknown".into());

    let token_status = match TokenChain::default().source() {
        Some(source) => format!("✓ ({source})"),
        None => "✗".into(),
    };

    println!("version:     {}", env!("CARGO_PKG_VERSION"));
    println!("user-agent:  {ua}");
    println!("config:      {config_path}");
    println!("base-url:    {}", config.base_url);
    println!("token:       {token_status}");

    Ok(())
}
