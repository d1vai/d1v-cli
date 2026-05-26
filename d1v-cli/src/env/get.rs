use std::io;

use anyhow::anyhow;
use d1v_api::api::projects::EnvVar;

use crate::text::{Field, Fields, Render, RenderContext, Span};
use crate::{Context, Result, t, theme};

use super::EnvGetArgs;
use super::list::format_value;

struct EnvVarDetail<'a> {
    var: &'a EnvVar,
    reveal: bool,
}

impl Render for EnvVarDetail<'_> {
    fn render(&self, ctx: &mut RenderContext<'_>) -> io::Result<()> {
        let label = theme::ansi::label();
        Fields::new([
            Field::new(
                Span::styled(t!("env-label-key"), label),
                Span::raw(self.var.key.clone()),
            ),
            Field::new(
                Span::styled(t!("env-label-value"), label),
                Span::raw(format_value(self.var, self.reveal).to_string()),
            ),
            Field::new(
                Span::styled(t!("env-label-description"), label),
                Span::raw(
                    self.var
                        .description
                        .clone()
                        .unwrap_or_else(|| "-".to_string()),
                ),
            ),
            Field::new(
                Span::styled(t!("env-label-sensitive"), label),
                Span::raw(if self.var.is_sensitive {
                    t!("env-yes")
                } else {
                    t!("env-no")
                }),
            ),
        ])
        .render(ctx)
    }
}

pub async fn run(ctx: &Context, args: EnvGetArgs) -> Result<()> {
    let project_id = args.project.resolve()?;
    let vars = ctx.client.project(&project_id).env().vars(true).await?;

    let var = if let Some(id) = args.id {
        vars.iter()
            .find(|v| v.id == id)
            .ok_or_else(|| anyhow!("{}", t!("env-key-not-found", key = id.to_string())))?
    } else {
        let key = args.key.as_deref().unwrap();
        vars.iter()
            .find(|v| v.key == key)
            .ok_or_else(|| anyhow!("{}", t!("env-key-not-found", key = key)))?
    };

    ctx.present(
        EnvVarDetail {
            var,
            reveal: args.reveal,
        },
        var,
    )
}
