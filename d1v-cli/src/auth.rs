use anyhow::Result;
use inquire::Text;
use tracing::debug;

use crate::token::TokenStore;
use crate::Context;

pub async fn login(ctx: &Context) -> Result<()> {
    let email = Text::new("Email:")
        .with_validator(|input: &str| {
            let valid = input
                .split_once('@')
                .is_some_and(|(user, domain)| !user.is_empty() && domain.contains('.'));

            if valid {
                Ok(inquire::validator::Validation::Valid)
            } else {
                Ok(inquire::validator::Validation::Invalid(
                    "please enter a valid email address".into(),
                ))
            }
        })
        .prompt()?;

    debug!("sending verification code");
    ctx.client.user().send_code(&email).await?;
    ctx.message(format_args!("Verification code sent to {email}"));

    let code = Text::new("Verification code:")
        .with_validator(|input: &str| {
            if input.len() == 6 && input.chars().all(|c| c.is_ascii_digit()) {
                Ok(inquire::validator::Validation::Valid)
            } else {
                Ok(inquire::validator::Validation::Invalid(
                    "please enter a 6-digit code".into(),
                ))
            }
        })
        .prompt()?;

    debug!("logging in");
    let token = ctx.client.user().login(&email, &code).await?;

    ctx.tokens.save(&token)?;
    ctx.client.token(token);
    debug!("login successful");
    ctx.message("Login successful!");

    Ok(())
}

pub async fn logout(ctx: &Context) -> Result<()> {
    ctx.tokens.delete()?;
    debug!("logged out");
    ctx.message("Logged out.");

    Ok(())
}
