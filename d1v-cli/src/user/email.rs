use crate::error::Result;
use tracing::debug;

use crate::{i18n, prompt, t, Context};

pub async fn bind(ctx: &Context) -> Result<()> {
    let pending = prompt::email_pending()?;
    let email = pending.value().to_string();

    debug!(%email, "binding email");
    pending
        .spin_ok(
            ctx.client
                .user()
                .send_bind_email_code(&email, i18n::locale()),
        )
        .await?;
    ctx.info(t!("email-code-sent", email = &email));

    let pending = prompt::code_pending()?;
    let code = pending.value().to_string();

    pending
        .spin_ok(ctx.client.user().confirm_bind_email(&email, &code))
        .await?;
    debug!(%email, "email bound");
    ctx.success(t!("email-bind-success"));

    Ok(())
}

pub async fn change(ctx: &Context) -> Result<()> {
    let pending = prompt::email_pending()?;
    let new_email = pending.value().to_string();

    debug!(email = %new_email, "changing email");
    pending
        .spin_ok(
            ctx.client
                .user()
                .send_change_email_code(&new_email, i18n::locale()),
        )
        .await?;
    ctx.info(t!("email-code-sent", email = &new_email));

    let pending = prompt::code_pending()?;
    let code = pending.value().to_string();

    pending
        .spin_ok(ctx.client.user().confirm_change_email(&new_email, &code))
        .await?;
    debug!(email = %new_email, "email changed");
    ctx.success(t!("email-change-success"));

    Ok(())
}
