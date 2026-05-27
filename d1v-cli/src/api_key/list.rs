use std::io;

use d1v_api::api::user::UserApiKey;

use crate::text::{Line, Render, RenderContext, Table, TableRow};
use crate::{Context, Result, t, theme};

struct ApiKeyList<'a>(&'a [UserApiKey]);

impl Render for ApiKeyList<'_> {
    fn render(&self, ctx: &mut RenderContext<'_>) -> io::Result<()> {
        if self.0.is_empty() {
            writeln!(ctx.writer, "{}", t!("api-key-empty-list"))?;
            return Ok(());
        }

        let rows = self.0.iter().map(|k| {
            TableRow::new([
                k.id.to_string(),
                k.name.clone(),
                k.key_prefix.clone(),
                k.description.clone().unwrap_or_else(|| "-".to_string()),
                k.created_at
                    .map(|t| t.strftime("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "-".to_string()),
                k.last_used_at
                    .map(|t| t.strftime("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "-".to_string()),
            ])
        });

        let header_style = theme::ansi::label();
        Table::new(rows)
            .header(TableRow::new([
                Line::styled(t!("api-key-label-id"), header_style),
                Line::styled(t!("api-key-label-name"), header_style),
                Line::styled(t!("api-key-label-prefix"), header_style),
                Line::styled(t!("api-key-label-description"), header_style),
                Line::styled(t!("api-key-label-created"), header_style),
                Line::styled(t!("api-key-label-last-used"), header_style),
            ]))
            .border_style(theme::ansi::border())
            .render(ctx)
    }
}

pub async fn run(ctx: &Context) -> Result<()> {
    let keys = ctx.client.user().api_keys().await?;
    ctx.present(ApiKeyList(&keys), &keys)
}
