mod auth;
mod config;
mod token;

use d1v_api::Client;
use std::sync::LazyLock;

use crate::config::Config;
use crate::token::{TokenChain, TokenLoader};

pub static CLIENT: LazyLock<Client> = LazyLock::new(|| {
    let config = Config::load().expect("failed to load config");

    let mut client =
        Client::new(reqwest::Client::new(), config.base_url).expect("invalid base URL");

    if let Ok(Some(token)) = TokenChain::default().load() {
        client.token(token);
    }

    client
});

fn main() {}
