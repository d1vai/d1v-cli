use std::io;

use anyhow::anyhow;
use d1v_api::api::user::UserApiKey;

use crate::text::{Field, Fields, Render, RenderContext, Span};
use crate::{Context, Result, t, theme};

use super::ApiKeyGetArgs;

struct ApiKeyDetail<'a>(&'a UserApiKey);

impl Render for ApiKeyDetail<'_> {
    fn render(&self, ctx: &mut RenderContext<'_>) -> io::Result<()> {
        let label = theme::ansi::label();
        Fields::new([
            Field::new(
                Span::styled(t!("api-key-label-id"), label),
                Span::raw(self.0.id.to_string()),
            ),
            Field::new(
                Span::styled(t!("api-key-label-name"), label),
                Span::raw(self.0.name.clone()),
            ),
            Field::new(
                Span::styled(t!("api-key-label-prefix"), label),
                Span::raw(self.0.key_prefix.clone()),
            ),
            Field::new(
                Span::styled(t!("api-key-label-description"), label),
                Span::raw(
                    self.0
                        .description
                        .clone()
                        .unwrap_or_else(|| "-".to_string()),
                ),
            ),
            Field::new(
                Span::styled(t!("api-key-label-created"), label),
                Span::raw(
                    self.0
                        .created_at
                        .map(|t| t.strftime("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_else(|| "-".to_string()),
                ),
            ),
            Field::new(
                Span::styled(t!("api-key-label-last-used"), label),
                Span::raw(
                    self.0
                        .last_used_at
                        .map(|t| t.strftime("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_else(|| "-".to_string()),
                ),
            ),
        ])
        .render(ctx)
    }
}

pub async fn run(ctx: &Context, args: ApiKeyGetArgs) -> Result<()> {
    let keys = ctx.client.user().api_keys().await?;

    let key = if let Some(id) = args.id {
        keys.iter()
            .find(|k| k.id == id)
            .ok_or_else(|| anyhow!("{}", t!("api-key-not-found-id", id = id.to_string())))?
    } else {
        let name = args.name.as_deref().unwrap();
        keys.iter()
            .find(|k| k.name == name)
            .ok_or_else(|| anyhow!("{}", t!("api-key-not-found-name", name = name)))?
    };

    ctx.present(ApiKeyDetail(key), key)
}
