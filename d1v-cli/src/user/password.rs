use anyhow::Result;
use inquire::Text;
use secrecy::SecretString;

use crate::t;
use crate::Context;

pub async fn set(ctx: &Context) -> Result<()> {
    let password = SecretString::from(
        inquire::Password::new(&t!("password-new-prompt"))
            .with_display_mode(inquire::PasswordDisplayMode::Masked)
            .prompt()?,
    );

    ctx.client.user().set_password(&password).await?;
    ctx.message(t!("password-set-success"));

    Ok(())
}

pub async fn reset(ctx: &Context) -> Result<()> {
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

    ctx.client.user().send_forgot_password_email(&email).await?;
    ctx.message(t!("password-forgot-sent", email = email));

    let code = Text::new(&t!("auth-code-prompt")).prompt()?;
    let new_password = SecretString::from(
        inquire::Password::new(&t!("password-new-prompt"))
            .with_display_mode(inquire::PasswordDisplayMode::Masked)
            .prompt()?,
    );

    ctx.client
        .user()
        .reset_password(&email, &code, &new_password)
        .await?;
    ctx.message(t!("password-reset-success"));

    Ok(())
}
