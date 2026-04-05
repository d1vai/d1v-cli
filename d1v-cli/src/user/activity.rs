use crate::error::Result;

use super::{ActivityArgs, ActivityTarget};
use crate::Context;

pub async fn run(ctx: &Context, args: ActivityArgs) -> Result<()> {
    let days = args.days;
    let api = ctx.client.user();

    let activity = match args.target {
        Some(ActivityTarget::User { user_id }) => {
            api.prompt_daily_activity_by_user(user_id, days).await?
        }
        Some(ActivityTarget::Slug { slug }) => {
            api.prompt_daily_activity_by_slug(&slug, days).await?
        }
        None => api.prompt_daily_activity(days).await?,
    };

    ctx.print(&activity)
}
