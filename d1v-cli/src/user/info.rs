use anyhow::Result;
use d1v_api::UpdateUser;

use super::{GetArgs, UpdateArgs};
use crate::t;
use crate::Context;

impl From<UpdateArgs> for UpdateUser {
    fn from(args: UpdateArgs) -> Self {
        Self {
            is_company: args.is_company,
            company_name: args.company_name,
            company_website: args.company_website,
            picture: args.picture,
            industry: args.industry,
            referral_code: args.referral_code,
        }
    }
}

pub async fn info(ctx: &Context) -> Result<()> {
    let user = ctx.client.user().info().await?;
    ctx.print(&user)
}

pub async fn update(ctx: &Context, args: UpdateArgs) -> Result<()> {
    let user = ctx.client.user().update_info(&args.into()).await?;
    ctx.message(t!("user-info-updated"));
    ctx.print(&user)
}

pub async fn get(ctx: &Context, target: GetArgs) -> Result<()> {
    let user = match target {
        GetArgs::Id { user_id } => ctx.client.user().public_user(user_id).await?,
        GetArgs::Slug { slug } => ctx.client.user().public_user_by_slug(&slug).await?,
    };

    ctx.print(&user)
}

pub async fn list(ctx: &Context) -> Result<()> {
    let users = ctx.client.user().all_users().await?;
    ctx.print_list(&users)
}
