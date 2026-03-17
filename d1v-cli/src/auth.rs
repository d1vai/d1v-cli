use anyhow::Result;
use inquire::Text;

use crate::Context;
use crate::token::TokenStore;

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

    ctx.client.user().send_code(&email).await?;
    println!("Verification code sent to {email}");

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

    let token = ctx.client.user().login(&email, &code).await?;

    ctx.tokens.save(&token)?;
    ctx.client.token(token);
    println!("Login successful!");

    Ok(())
}

pub async fn logout(ctx: &Context) -> Result<()> {
    ctx.tokens.delete()?;
    println!("Logged out.");

    Ok(())
}
