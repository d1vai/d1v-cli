use crate::error::Result;
use tracing::debug;

use crate::{i18n, prompt, t, Context};

pub async fn set(ctx: &Context) -> Result<()> {
    let password = prompt::new_password()?;

    debug!("setting password");
    ctx.client.user().set_password(&password).await?;
    debug!("password set");
    ctx.success(t!("password-set-success"));

    Ok(())
}

pub async fn reset(ctx: &Context) -> Result<()> {
    let pending = prompt::email_pending()?;
    let email = pending.value().to_string();

    debug!(%email, "resetting password");
    pending
        .spin_ok(
            ctx.client
                .user()
                .send_forgot_password_email(&email, i18n::locale()),
        )
        .await?;
    ctx.message(t!("password-forgot-sent", email = &email));

    let code = prompt::code()?;
    let new_password = prompt::new_password()?;

    ctx.client
        .user()
        .reset_password(&email, &code, &new_password)
        .await?;
    debug!(%email, "password reset");
    ctx.success(t!("password-reset-success"));

    Ok(())
}
