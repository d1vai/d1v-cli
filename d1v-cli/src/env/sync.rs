use std::io;

use anyhow::anyhow;
use d1v_api::api::projects::SyncEnvVarsResponse;

use crate::text::{Field, Fields, Render, RenderContext, Span};
use crate::{Context, Result, t, theme};

use super::EnvSyncArgs;

struct SyncSummary<'a>(&'a SyncEnvVarsResponse);

impl Render for SyncSummary<'_> {
    fn render(&self, ctx: &mut RenderContext<'_>) -> io::Result<()> {
        let label = theme::ansi::label();
        Fields::new([
            Field::new(
                Span::styled(t!("env-label-message"), label),
                Span::raw(self.0.message.clone()),
            ),
            Field::new(
                Span::styled(t!("env-label-dev-project"), label),
                Span::raw(self.0.vercel_dev_project_id.clone()),
            ),
            Field::new(
                Span::styled(t!("env-label-dev-env-count"), label),
                Span::raw(self.0.dev_local_env_count.to_string()),
            ),
            Field::new(
                Span::styled(t!("env-label-dev-up-to-date"), label),
                Span::raw(if self.0.dev_up_to_date {
                    t!("env-yes")
                } else {
                    t!("env-no")
                }),
            ),
            Field::new(
                Span::styled(t!("env-label-prod-project"), label),
                Span::raw(self.0.vercel_prod_project_id.clone()),
            ),
            Field::new(
                Span::styled(t!("env-label-prod-env-count"), label),
                Span::raw(self.0.prod_local_env_count.to_string()),
            ),
            Field::new(
                Span::styled(t!("env-label-prod-up-to-date"), label),
                Span::raw(if self.0.prod_up_to_date {
                    t!("env-yes")
                } else {
                    t!("env-no")
                }),
            ),
        ])
        .render(ctx)
    }
}

pub async fn run(ctx: &Context, args: EnvSyncArgs) -> Result<()> {
    if !args.yes {
        return Err(anyhow!("{}", t!("env-sync-confirm-required")).into());
    }
    let project_id = args.project.resolve()?;
    let result = ctx.client.project(&project_id).env().sync_vercel().await?;
    ctx.present(SyncSummary(&result), &result)
}
