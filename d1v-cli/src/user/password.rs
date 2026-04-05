use crate::error::Result;
use tracing::debug;

use crate::{prompt, t, Context};

pub async fn set(ctx: &Context) -> Result<()> {
    let password = prompt::new_password()?;

    debug!("setting password");
    ctx.client.user().set_password(&password).await?;
    debug!("password set");
    ctx.message(t!("password-set-success"));

    Ok(())
}

pub async fn reset(ctx: &Context) -> Result<()> {
    let email = prompt::email()?;

    debug!(%email, "resetting password");
    ctx.client.user().send_forgot_password_email(&email).await?;
    ctx.message(t!("password-forgot-sent", email = email));

    let code = prompt::code()?;
    let new_password = prompt::new_password()?;

    ctx.client
        .user()
        .reset_password(&email, &code, &new_password)
        .await?;
    debug!(%email, "password reset");
    ctx.message(t!("password-reset-success"));

    Ok(())
}
