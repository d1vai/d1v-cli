use crate::error::Result;
use tracing::debug;

use crate::{prompt, t, Context};

pub async fn bind(ctx: &Context) -> Result<()> {
    let email = prompt::email()?;

    debug!(%email, "binding email");
    ctx.client.user().send_bind_email_code(&email).await?;
    ctx.message(t!("email-code-sent", email = email));

    let code = prompt::code()?;

    ctx.client.user().confirm_bind_email(&email, &code).await?;
    debug!(%email, "email bound");
    ctx.message(t!("email-bind-success"));

    Ok(())
}

pub async fn change(ctx: &Context) -> Result<()> {
    let new_email = prompt::email()?;

    debug!(email = %new_email, "changing email");
    ctx.client.user().send_change_email_code(&new_email).await?;
    ctx.message(t!("email-code-sent", email = new_email));

    let code = prompt::code()?;

    ctx.client
        .user()
        .confirm_change_email(&new_email, &code)
        .await?;
    debug!(email = %new_email, "email changed");
    ctx.message(t!("email-change-success"));

    Ok(())
}
