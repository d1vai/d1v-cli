use anyhow::Result;
use inquire::Text;
use secrecy::SecretString;
use tracing::debug;

use crate::t;
use crate::token::TokenStore;
use crate::Context;

pub async fn login(ctx: &Context, password: bool) -> Result<()> {
    let email = prompt_email()?;

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

fn prompt_email() -> Result<String> {
    let email = Text::new(&t!("auth-email-prompt"))
        .with_validator(|input: &str| {
            let valid = input
                .split_once('@')
                .is_some_and(|(user, domain)| !user.is_empty() && domain.contains('.'));

            if valid {
                Ok(inquire::validator::Validation::Valid)
            } else {
                Ok(inquire::validator::Validation::Invalid(
                    t!("auth-email-invalid").into(),
                ))
            }
        })
        .prompt()?;

    Ok(email)
}

async fn authenticate_code(ctx: &Context, email: &str) -> Result<SecretString> {
    debug!("sending verification code");
    ctx.client.user().send_code(email).await?;
    ctx.message(t!("auth-code-sent", email = email));

    let code = Text::new(&t!("auth-code-prompt"))
        .with_validator(|input: &str| {
            if input.len() == 6 && input.chars().all(|c| c.is_ascii_digit()) {
                Ok(inquire::validator::Validation::Valid)
            } else {
                Ok(inquire::validator::Validation::Invalid(
                    t!("auth-code-invalid").into(),
                ))
            }
        })
        .prompt()?;

    debug!("logging in with verification code");
    Ok(ctx.client.user().login(email, &code).await?)
}

async fn authenticate_password(ctx: &Context, email: &str) -> Result<SecretString> {
    let password = inquire::Password::new(&t!("auth-password-prompt"))
        .without_confirmation()
        .prompt()?;

    debug!("logging in with password");
    Ok(ctx.client.user().login_password(email, &password).await?)
}

pub async fn logout(ctx: &Context) -> Result<()> {
    ctx.tokens.delete()?;
    debug!("logged out");
    ctx.message(t!("auth-logout-success"));

    Ok(())
}
