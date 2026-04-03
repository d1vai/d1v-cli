use std::io::{stdin, IsTerminal};

use anyhow::Result;
use secrecy::SecretString;
use tracing::debug;

use crate::token::{TokenLoader, TokenStore};
use crate::ui::Password;
use crate::{prompt, t, Context};

pub async fn login(ctx: &Context, password: bool) -> Result<()> {
    let email = prompt::email()?;

    let token = if password {
        authenticate_password(ctx, &email).await?
    } else {
        authenticate_code(ctx, &email).await?
    };

    ctx.tokens.save(&token)?;
    ctx.client.token(token);
    debug!("login successful");
    ctx.message(t!("auth-login-success"));

    Ok(())
}

pub fn login_with_token(ctx: &Context) -> Result<()> {
    let token = if stdin().is_terminal() {
        Password::new(t!("auth-token-prompt")).prompt()?
    } else {
        let mut buf = String::new();
        stdin().read_line(&mut buf)?;
        SecretString::from(buf.trim())
    };

    ctx.tokens.save(&token)?;
    ctx.client.token(token);
    debug!("login with token successful");
    ctx.message(t!("auth-login-success"));

    Ok(())
}

async fn authenticate_code(ctx: &Context, email: &str) -> Result<SecretString> {
    debug!("sending verification code");
    ctx.client.user().send_code(email).await?;
    ctx.message(t!("auth-code-sent", email = email));

    let code = prompt::code()?;

    debug!("logging in with verification code");
    Ok(ctx.client.user().login(email, &code).await?)
}

async fn authenticate_password(ctx: &Context, email: &str) -> Result<SecretString> {
    let password = Password::new(t!("auth-password-prompt")).prompt()?;

    debug!("logging in with password");
    Ok(ctx.client.user().login_password(email, &password).await?)
}

pub async fn logout(ctx: &Context) -> Result<()> {
    if ctx.tokens.load()?.is_none() {
        ctx.message(t!("auth-not-logged-in"));
        return Ok(());
    }

    ctx.tokens.delete()?;
    debug!("logged out");
    ctx.message(t!("auth-logout-success"));

    Ok(())
}
