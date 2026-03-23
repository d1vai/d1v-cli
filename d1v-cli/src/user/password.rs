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

pub async fn forgot(ctx: &Context) -> Result<()> {
    let email = Text::new(&t!("auth-email-prompt")).prompt()?;

    ctx.client.user().send_forgot_password_email(&email).await?;
    ctx.message(t!("password-forgot-sent", email = email));

    Ok(())
}

pub async fn reset(ctx: &Context) -> Result<()> {
    let email = Text::new(&t!("auth-email-prompt")).prompt()?;
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
