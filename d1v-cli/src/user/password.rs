use anyhow::Result;
use secrecy::SecretString;

use crate::{prompt, t, Context};

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
    let email = prompt::email()?;

    ctx.client.user().send_forgot_password_email(&email).await?;
    ctx.message(t!("password-forgot-sent", email = email));

    let code = prompt::code()?;
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
