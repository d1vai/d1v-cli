use anyhow::Result;

use crate::{t, Context};

pub async fn accept(ctx: &Context, invite_code: &str) -> Result<()> {
    ctx.client.user().accept_invitation(invite_code).await?;
    ctx.message(t!("invitation-accepted"));

    Ok(())
}

pub async fn list(ctx: &Context) -> Result<()> {
    let invitees = ctx.client.user().list_invitees().await?;

    for user in &invitees {
        ctx.print(user)?;
    }

    Ok(())
}
