use crate::theme::ansi::{Stream, Stylize};
use colorgrad::{GradientBuilder, LinearGradient};
use d1v_api::{UpdateUser, User};
use serde::Serialize;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::sync::LazyLock;
use tracing::debug;

use super::{write_row, GetArgs, UpdateArgs, LABEL_WIDTH};
use crate::error::Result;
use crate::output::pad_label;
use crate::text::{Line, Render, RenderContext, Table, TableRow, Text as TextBlock};
use crate::ui::{Select, SelectOption, Text};
use crate::Context;
use crate::{t, theme};

pub struct UserListItemView<'a>(pub &'a User);

impl Render for UserListItemView<'_> {
    fn render(&self, ctx: &mut RenderContext<'_>) -> std::io::Result<()> {
        TextBlock::from(self.line()).render(ctx)
    }
}

impl UserListItemView<'_> {
    fn line(&self) -> Line {
        let mut line = Line::styled(self.0.id.to_string(), theme::ansi::dim());

        if !self.0.slug.is_empty() {
            line = line
                .push_plain(" ")
                .push_styled(self.0.slug.clone(), theme::ansi::plain());
        }

        if let Some(email) = &self.0.email
            && !email.is_empty()
        {
            line = line
                .push_plain(" ")
                .push_styled(format!("<{email}>"), theme::ansi::plain());
        }

        line
    }
}

pub struct UserListView<'a>(pub &'a [User]);

impl Render for UserListView<'_> {
    fn render(&self, ctx: &mut RenderContext<'_>) -> std::io::Result<()> {
        Table::new(self.0.iter().map(Self::row))
            .header(Self::header())
            .border_style(theme::ansi::border())
            .render(ctx)
    }
}

impl UserListView<'_> {
    fn header() -> TableRow {
        TableRow::new([
            Line::styled("id", theme::ansi::label()),
            Line::styled("slug", theme::ansi::label()),
            Line::styled("email", theme::ansi::label()),
        ])
    }

    fn row(user: &User) -> TableRow {
        TableRow::new([
            Line::styled(user.id.to_string(), theme::ansi::dim()),
            Line::styled(user.slug.clone(), theme::ansi::plain()),
            Line::styled(user.email.clone().unwrap_or_default(), theme::ansi::plain()),
        ])
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

#[derive(Serialize)]
#[serde(transparent)]
struct UserDetail<'a>(&'a User);

static SUPER_ADMIN_GRADIENT: LazyLock<LinearGradient> = LazyLock::new(|| {
    GradientBuilder::new()
        .html_colors(&["#C83CFF", "#FFC83C"])
        .build()
        .expect("valid gradient colors")
});

fn format_roles(user: &User) -> String {
    let mut roles: Vec<String> = Vec::new();

    if user.is_super_admin {
        roles.push(theme::ansi::gradient("super-admin", &*SUPER_ADMIN_GRADIENT));
    }

    if user.is_admin {
        roles.push(
            "admin"
                .if_supports_color(Stream::Stdout, |s| s.style(theme::ansi::error()))
                .to_string(),
        );
    }

    if user.is_agent {
        roles.push(
            "agent"
                .if_supports_color(Stream::Stdout, |s| s.style(theme::ansi::warning()))
                .to_string(),
        );
    }

    if roles.is_empty() {
        roles.push("user".to_string());
    }

    roles.join(", ")
}

impl Display for UserDetail<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let user = self.0;

        write!(
            f,
            "{}{}",
            pad_label(t!("user-label-id"), LABEL_WIDTH)
                .if_supports_color(Stream::Stdout, |s| s.style(theme::ansi::label())),
            user.id
                .if_supports_color(Stream::Stdout, |s| s.style(theme::ansi::value())),
        )?;

        if !user.slug.is_empty() {
            write_row(f, "user-label-slug", &user.slug)?;
        }

        if let Some(email) = &user.email
            && !email.is_empty()
        {
            write_row(f, "user-label-email", email)?;
        }

        write!(
            f,
            "\n{}{}",
            pad_label(t!("user-label-roles"), LABEL_WIDTH)
                .if_supports_color(Stream::Stdout, |s| s.style(theme::ansi::label())),
            format_roles(user),
        )?;

        if user.is_company {
            if !user.company_name.is_empty() {
                write_row(f, "user-label-company", &user.company_name)?;
            }
            if !user.company_website.is_empty() {
                write_row(f, "user-label-website", &user.company_website)?;
            }
        }

        if !user.industry.is_empty() {
            write_row(f, "user-label-industry", &user.industry)?;
        }

        if !user.invite_code.is_empty() {
            write_row(f, "user-label-invite-code", &user.invite_code)?;
        }

        Ok(())
    }
}

pub async fn info(ctx: &Context) -> Result<()> {
    let user = ctx.client.user().info().await?;
    ctx.print(&UserDetail(&user))
}

pub async fn update(ctx: &Context, args: UpdateArgs) -> Result<()> {
    debug!("updating user info");
    let user = ctx.client.user().update_info(&args.into()).await?;
    ctx.success(t!("user-info-updated"));
    ctx.print(&UserDetail(&user))
}

pub async fn update_interactive(ctx: &Context) -> Result<()> {
    enum Field {
        CompanyName,
        CompanyWebsite,
        Picture,
        Industry,
        ReferralCode,
    }

    let field = Select::new(t!("user-update-field-prompt"))
        .option(SelectOption::new(
            Field::CompanyName,
            t!("user-update-field-company-name"),
        ))
        .option(SelectOption::new(
            Field::CompanyWebsite,
            t!("user-update-field-company-website"),
        ))
        .option(SelectOption::new(
            Field::Picture,
            t!("user-update-field-picture"),
        ))
        .option(SelectOption::new(
            Field::Industry,
            t!("user-update-field-industry"),
        ))
        .option(SelectOption::new(
            Field::ReferralCode,
            t!("user-update-field-referral-code"),
        ))
        .prompt()?;

    let label = match field {
        Field::CompanyName => t!("user-update-field-company-name"),
        Field::CompanyWebsite => t!("user-update-field-company-website"),
        Field::Picture => t!("user-update-field-picture"),
        Field::Industry => t!("user-update-field-industry"),
        Field::ReferralCode => t!("user-update-field-referral-code"),
    };

    let value = Text::new(format!("{label}:")).prompt()?;

    let mut update = UpdateUser::default();
    match field {
        Field::CompanyName => update.company_name = Some(value),
        Field::CompanyWebsite => update.company_website = Some(value),
        Field::Picture => update.picture = Some(value),
        Field::Industry => update.industry = Some(value),
        Field::ReferralCode => update.referral_code = Some(value),
    }

    debug!("updating user info (interactive)");
    let user = ctx.client.user().update_info(&update).await?;
    ctx.success(t!("user-info-updated"));
    ctx.print(&UserDetail(&user))
}

pub async fn get(ctx: &Context, target: GetArgs) -> Result<()> {
    let user = match target {
        GetArgs::Id { user_id } => ctx.client.user().public_user(user_id).await?,
        GetArgs::Slug { slug } => ctx.client.user().public_user_by_slug(&slug).await?,
    };

    ctx.present(UserListItemView(&user), &user)
}

pub async fn list(ctx: &Context) -> Result<()> {
    let users = ctx.client.user().all_users().await?;
    ctx.present(UserListView(&users), &users)
}
