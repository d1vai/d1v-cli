use crate::error::Result;
use tracing::debug;

use super::info::UserListView;
use crate::{Context, t};

pub async fn accept(ctx: &Context, invite_code: &str) -> Result<()> {
    debug!(%invite_code, "accepting invitation");
    ctx.client.user().accept_invitation(invite_code).await?;
    debug!(%invite_code, "invitation accepted");
    ctx.success(t!("invitation-accepted"));

    Ok(())
}

pub async fn list(ctx: &Context) -> Result<()> {
    let invitees = ctx.client.user().list_invitees().await?;
    ctx.present(UserListView(&invitees), &invitees)
}
