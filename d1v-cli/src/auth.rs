use anyhow::Result;
use inquire::Text;

use crate::token::{TokenChain, TokenStore};
use crate::CLIENT;

pub async fn login() -> Result<()> {
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

    CLIENT.user().send_code(&email).await?;
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

    let token = CLIENT.user().login(&email, &code).await?;

    TokenChain::default().save(&token)?;
    println!("Login successful!");

    Ok(())
}

pub async fn logout() -> Result<()> {
    TokenChain::default().delete()?;
    println!("Logged out.");

    Ok(())
}
