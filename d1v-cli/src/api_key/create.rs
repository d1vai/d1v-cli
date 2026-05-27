use std::io::{self, IsTerminal};

use anyhow::anyhow;
use secrecy::ExposeSecret;

use crate::text::{Line, Render, RenderContext};
use crate::token::TokenStore;
use crate::ui::{Select, SelectOption, Text};
use crate::{Context, Result, t};

use super::ApiKeyCreateArgs;

struct CreateResult {
    name: String,
    key_prefix: String,
    api_key_plain: String,
}

impl Render for CreateResult {
    fn render(&self, ctx: &mut RenderContext<'_>) -> io::Result<()> {
        use crate::theme;

        Line::raw(t!(
            "api-key-create-success",
            name = self.name.clone(),
            prefix = self.key_prefix.clone()
        ))
        .render(ctx)?;

        writeln!(ctx.writer)?;
        Line::styled(self.api_key_plain.clone(), theme::ansi::warning()).render(ctx)?;
        writeln!(ctx.writer)?;
        Line::raw(t!("api-key-create-once-warning")).render(ctx)?;
        writeln!(ctx.writer)
    }
}

pub async fn run(ctx: &Context, args: ApiKeyCreateArgs) -> Result<()> {
    let (name, description) = if let Some(name) = args.name {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err(anyhow!("{}", t!("api-key-create-name-empty")).into());
        }
        (name, args.description)
    } else if io::stdin().is_terminal() {
        let name = Text::new(t!("api-key-create-name-prompt"))
            .with_validator(|s| {
                if s.trim().is_empty() {
                    Err(t!("api-key-create-name-empty"))
                } else {
                    Ok(())
                }
            })
            .prompt()?
            .trim()
            .to_string();

        let desc = Text::new(t!("api-key-create-desc-prompt")).prompt()?;
        let desc = if desc.trim().is_empty() {
            None
        } else {
            Some(desc.trim().to_string())
        };

        (name, desc)
    } else {
        return Err(anyhow!("{}", t!("api-key-create-name-required")).into());
    };

    let result = ctx
        .client
        .user()
        .create_api_key(&name, description.as_deref())
        .await?;

    let api_key_plain = result.api_key.expose_secret().to_string();
    let view = CreateResult {
        name: result.item.name.clone(),
        key_prefix: result.item.key_prefix.clone(),
        api_key_plain,
    };

    ctx.present(view, &result.item)?;

    if io::stdin().is_terminal() {
        prompt_save(ctx, &result).await?;
    }

    Ok(())
}

async fn prompt_save(ctx: &Context, result: &d1v_api::api::user::CreatedApiKey) -> Result<()> {
    enum SaveChoice {
        Save,
        Skip,
    }

    let choice = Select::new(t!("api-key-save-prompt"))
        .option(SelectOption::new(SaveChoice::Save, t!("api-key-save-yes")))
        .option(SelectOption::new(SaveChoice::Skip, t!("api-key-save-skip")))
        .default_index(0)
        .prompt()?;

    if matches!(choice, SaveChoice::Save) {
        ctx.tokens.save(&result.api_key)?;
        ctx.client.token(result.api_key.clone());
        ctx.success(t!("api-key-save-saved"));
    }

    Ok(())
}
