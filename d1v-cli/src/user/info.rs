use crate::error::Result;
use d1v_api::{UpdateUser, User};
use owo_colors::{OwoColorize, Stream};
use serde::Serialize;
use std::fmt;
use std::fmt::{Display, Formatter};
use tracing::debug;

use super::{GetArgs, UpdateArgs};
use crate::t;
use crate::Context;

#[derive(Serialize)]
#[serde(transparent)]
pub struct UserListItem<'a>(pub &'a User);

impl Display for UserListItem<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.0.id.if_supports_color(Stream::Stdout, |s| s.dimmed())
        )?;

        if !self.0.slug.is_empty() {
            write!(
                f,
                " {}",
                self.0.slug.if_supports_color(Stream::Stdout, |s| s.bold())
            )?;
        }

        if let Some(email) = &self.0.email
            && !email.is_empty()
        {
            write!(
                f,
                " {}",
                format!("<{email}>").if_supports_color(Stream::Stdout, |s| s.cyan())
            )?;
        }

        Ok(())
    }
}

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
    debug!("updating user info");
    let user = ctx.client.user().update_info(&args.into()).await?;
    ctx.success(t!("user-info-updated"));
    ctx.print(&user)
}

pub async fn get(ctx: &Context, target: GetArgs) -> Result<()> {
    let user = match target {
        GetArgs::Id { user_id } => ctx.client.user().public_user(user_id).await?,
        GetArgs::Slug { slug } => ctx.client.user().public_user_by_slug(&slug).await?,
    };

    ctx.print(&UserListItem(&user))
}

pub async fn list(ctx: &Context) -> Result<()> {
    let users = ctx.client.user().all_users().await?;
    ctx.print_list(users.iter().map(UserListItem))
}
