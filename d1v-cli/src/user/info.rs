use colorgrad::{GradientBuilder, LinearGradient};
use d1v_api::{UpdateUser, User};
use std::sync::LazyLock;
use tracing::debug;

use super::{GetArgs, UpdateArgs};
use crate::Context;
use crate::error::Result;
use crate::text::{
    Field, Fields, Line, Render, RenderContext, Span, Table, TableRow, Text as TextBlock,
};
use crate::ui::{Select, SelectOption, Text};
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

struct UserDetailView<'a>(&'a User);

static SUPER_ADMIN_GRADIENT: LazyLock<LinearGradient> = LazyLock::new(|| {
    GradientBuilder::new()
        .html_colors(&["#C83CFF", "#FFC83C"])
        .build()
        .expect("valid gradient colors")
});

impl UserDetailView<'_> {
    fn field(label: String, value: impl Into<Line>) -> Field {
        Field::new(Span::styled(label, theme::ansi::label()), value)
    }

    fn roles(&self) -> Line {
        let user = self.0;
        let mut roles: Vec<Line> = Vec::new();

        if user.is_super_admin {
            roles.push(theme::ansi::gradient_line(
                "super-admin",
                &*SUPER_ADMIN_GRADIENT,
            ));
        }

        if user.is_admin {
            roles.push(Line::styled("admin", theme::ansi::error()));
        }

        if user.is_agent {
            roles.push(Line::styled("agent", theme::ansi::warning()));
        }

        if roles.is_empty() {
            return Line::raw("user");
        }

        let mut line = Line::new();
        for (index, role) in roles.into_iter().enumerate() {
            if index > 0 {
                line = line.push_plain(", ");
            }
            line = line.extend(role);
        }

        line
    }

    fn push_text_field(fields: &mut Vec<Field>, label: String, value: &str) {
        if !value.is_empty() {
            fields.push(Self::field(
                label,
                Span::styled(value.to_owned(), theme::ansi::value()),
            ));
        }
    }
}

impl Render for UserDetailView<'_> {
    fn render(&self, ctx: &mut RenderContext<'_>) -> std::io::Result<()> {
        let user = self.0;
        let mut fields = vec![Self::field(
            t!("user-label-id"),
            Span::styled(user.id.to_string(), theme::ansi::value()),
        )];

        Self::push_text_field(&mut fields, t!("user-label-slug"), &user.slug);

        if let Some(email) = &user.email {
            Self::push_text_field(&mut fields, t!("user-label-email"), email);
        }

        fields.push(Self::field(t!("user-label-roles"), self.roles()));

        if user.is_company {
            Self::push_text_field(&mut fields, t!("user-label-company"), &user.company_name);
            Self::push_text_field(&mut fields, t!("user-label-website"), &user.company_website);
        }

        Self::push_text_field(&mut fields, t!("user-label-industry"), &user.industry);
        Self::push_text_field(&mut fields, t!("user-label-invite-code"), &user.invite_code);

        Fields::new(fields).render(ctx)
    }
}

pub async fn info(ctx: &Context) -> Result<()> {
    let user = ctx.client.user().info().await?;
    ctx.present(UserDetailView(&user), &user)
}

pub async fn update(ctx: &Context, args: UpdateArgs) -> Result<()> {
    debug!("updating user info");
    let user = ctx.client.user().update_info(&args.into()).await?;
    ctx.success(t!("user-info-updated"));
    ctx.present(UserDetailView(&user), &user)
}

pub async fn update_interactive(ctx: &Context) -> Result<()> {
    enum UpdateField {
        CompanyName,
        CompanyWebsite,
        Picture,
        Industry,
        ReferralCode,
    }

    let field = Select::new(t!("user-update-field-prompt"))
        .option(SelectOption::new(
            UpdateField::CompanyName,
            t!("user-update-field-company-name"),
        ))
        .option(SelectOption::new(
            UpdateField::CompanyWebsite,
            t!("user-update-field-company-website"),
        ))
        .option(SelectOption::new(
            UpdateField::Picture,
            t!("user-update-field-picture"),
        ))
        .option(SelectOption::new(
            UpdateField::Industry,
            t!("user-update-field-industry"),
        ))
        .option(SelectOption::new(
            UpdateField::ReferralCode,
            t!("user-update-field-referral-code"),
        ))
        .prompt()?;

    let label = match field {
        UpdateField::CompanyName => t!("user-update-field-company-name"),
        UpdateField::CompanyWebsite => t!("user-update-field-company-website"),
        UpdateField::Picture => t!("user-update-field-picture"),
        UpdateField::Industry => t!("user-update-field-industry"),
        UpdateField::ReferralCode => t!("user-update-field-referral-code"),
    };

    let value = Text::new(format!("{label}:")).prompt()?;

    let mut update = UpdateUser::default();
    match field {
        UpdateField::CompanyName => update.company_name = Some(value),
        UpdateField::CompanyWebsite => update.company_website = Some(value),
        UpdateField::Picture => update.picture = Some(value),
        UpdateField::Industry => update.industry = Some(value),
        UpdateField::ReferralCode => update.referral_code = Some(value),
    }

    debug!("updating user info (interactive)");
    let user = ctx.client.user().update_info(&update).await?;
    ctx.success(t!("user-info-updated"));
    ctx.present(UserDetailView(&user), &user)
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
