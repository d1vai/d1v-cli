use anyhow::Result;
use secrecy::SecretString;
use tracing::debug;

use crate::token::TokenStore;
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

async fn authenticate_code(ctx: &Context, email: &str) -> Result<SecretString> {
    debug!("sending verification code");
    ctx.client.user().send_code(email).await?;
    ctx.message(t!("auth-code-sent", email = email));

    let code = prompt::code()?;

    debug!("logging in with verification code");
    Ok(ctx.client.user().login(email, &code).await?)
}

async fn authenticate_password(ctx: &Context, email: &str) -> Result<SecretString> {
    let password = SecretString::from(
        inquire::Password::new(&t!("auth-password-prompt"))
            .without_confirmation()
            .prompt()?,
    );

    debug!("logging in with password");
    Ok(ctx.client.user().login_password(email, &password).await?)
}

pub async fn logout(ctx: &Context) -> Result<()> {
    ctx.tokens.delete()?;
    debug!("logged out");
    ctx.message(t!("auth-logout-success"));

    Ok(())
}
