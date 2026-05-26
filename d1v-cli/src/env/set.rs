use std::io;

use d1v_api::api::projects::EnvVar;
use serde::Serialize;

use crate::text::{Line, Render, RenderContext};
use crate::{Context, Result, t};

use super::EnvSetArgs;

struct SetSummary {
    created: usize,
    updated: usize,
}

impl Render for SetSummary {
    fn render(&self, ctx: &mut RenderContext<'_>) -> io::Result<()> {
        Line::raw(t!(
            "env-set-summary",
            created = self.created,
            updated = self.updated
        ))
        .render(ctx)
    }
}

#[derive(Serialize)]
struct SetResultJson<'a> {
    created: &'a [EnvVar],
    updated: &'a [EnvVar],
}

pub async fn run(ctx: &Context, args: EnvSetArgs) -> Result<()> {
    let project_id = args.project.resolve()?;
    let env = ctx.client.project(&project_id).env();
    let existing = env.vars(false).await?;

    let mut created: Vec<EnvVar> = Vec::new();
    let mut updated: Vec<EnvVar> = Vec::new();

    for entry in &args.vars {
        let desc = args.description.as_deref();
        let sensitive = args.sensitive.then_some(true);

        if let Some(existing_var) = existing.iter().find(|v| v.key == entry.key()) {
            let var = env
                .update_var(existing_var.id)
                .maybe_value(Some(entry.value()))
                .maybe_description(desc)
                .maybe_is_sensitive(sensitive)
                .call()
                .await?;
            updated.push(var);
        } else {
            let var = env
                .create_var(entry.key(), entry.value())
                .maybe_description(desc)
                .maybe_is_sensitive(sensitive)
                .call()
                .await?;
            created.push(var);
        }
    }

    ctx.present(
        SetSummary {
            created: created.len(),
            updated: updated.len(),
        },
        &SetResultJson {
            created: &created,
            updated: &updated,
        },
    )
}
