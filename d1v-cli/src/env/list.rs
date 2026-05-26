use std::io;

use d1v_api::api::projects::EnvVar;

use crate::text::{Line, Render, RenderContext, Table, TableRow};
use crate::{Context, Result, t, theme};

use super::EnvListArgs;

pub fn format_value(var: &EnvVar, reveal: bool) -> &str {
    if reveal {
        var.value.as_deref().unwrap_or("-")
    } else if var.is_sensitive {
        &var.value_preview
    } else {
        var.value.as_deref().unwrap_or("-")
    }
}

struct EnvVarList<'a> {
    vars: &'a [EnvVar],
    reveal: bool,
}

impl Render for EnvVarList<'_> {
    fn render(&self, ctx: &mut RenderContext<'_>) -> io::Result<()> {
        if self.vars.is_empty() {
            writeln!(ctx.writer, "{}", t!("env-empty-list"))?;
            return Ok(());
        }
        let rows = self.vars.iter().map(|var| {
            TableRow::new([
                var.key.clone(),
                format_value(var, self.reveal).to_string(),
                var.description.clone().unwrap_or_else(|| "-".to_string()),
                if var.is_sensitive {
                    t!("env-yes")
                } else {
                    t!("env-no")
                },
            ])
        });

        let header_style = theme::ansi::label();
        Table::new(rows)
            .header(TableRow::new([
                Line::styled(t!("env-label-key"), header_style),
                Line::styled(t!("env-label-value"), header_style),
                Line::styled(t!("env-label-description"), header_style),
                Line::styled(t!("env-label-sensitive"), header_style),
            ]))
            .border_style(theme::ansi::border())
            .render(ctx)
    }
}

pub async fn run(ctx: &Context, args: EnvListArgs) -> Result<()> {
    let project_id = args.project.resolve()?;
    let vars = ctx
        .client
        .project(&project_id)
        .env()
        .vars(args.reveal)
        .await?;

    ctx.present(
        EnvVarList {
            vars: &vars,
            reveal: args.reveal,
        },
        &vars,
    )
}
