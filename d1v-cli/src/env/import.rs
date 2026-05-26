use std::fs;
use std::io::{self, IsTerminal, Read};

use anyhow::anyhow;
use d1v_api::api::projects::ImportEnvVarsResponse;

use crate::text::{Line, Render, RenderContext};
use crate::{Context, Result, t};

use super::EnvImportArgs;

struct ImportSummary(ImportEnvVarsResponse);

impl Render for ImportSummary {
    fn render(&self, ctx: &mut RenderContext<'_>) -> io::Result<()> {
        Line::raw(t!(
            "env-import-summary",
            created = self.0.created,
            updated = self.0.updated,
            skipped = self.0.skipped,
            total = self.0.total,
        ))
        .render(ctx)
    }
}

pub async fn run(ctx: &Context, args: EnvImportArgs) -> Result<()> {
    let project_id = args.project.resolve()?;
    let content = if let Some(path) = &args.input {
        fs::read_to_string(path)?
    } else {
        if io::stdin().is_terminal() {
            return Err(anyhow!("{}", t!("env-import-stdin-required")).into());
        }
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        buf
    };

    let result = ctx
        .client
        .project(&project_id)
        .env()
        .import_vars(&content, args.overwrite)
        .await?;
    ctx.present(ImportSummary(result.clone()), &result)
}
