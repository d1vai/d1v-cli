use std::fs;

use crate::text::Line;
use crate::{Context, Result, t};

use super::EnvExportArgs;

pub async fn run(ctx: &Context, args: EnvExportArgs) -> Result<()> {
    let project_id = args.project.resolve()?;
    let result = ctx.client.project(&project_id).env().export_vars().await?;

    if let Some(path) = &args.output {
        fs::write(path, &result.content)?;
        ctx.success(t!("env-export-saved", path = path));
        Ok(())
    } else {
        ctx.present(Line::raw(result.content.clone()), &result)
    }
}
