use crate::{Context, Result, t};
use anyhow::anyhow;

use super::EnvUnsetArgs;

pub async fn run(ctx: &Context, args: EnvUnsetArgs) -> Result<()> {
    let project_id = args.project.resolve()?;
    let env = ctx.client.project(&project_id).env();

    let id = if let Some(id) = args.id {
        id
    } else {
        let key = args.key.as_deref().unwrap();
        let vars = env.vars(false).await?;
        vars.iter()
            .find(|v| &v.key == key)
            .map(|v| v.id)
            .ok_or_else(|| anyhow!("{}", t!("env-key-not-found", key = key)))?
    };

    let message = env.delete_var(id).await?;
    ctx.success(message);
    Ok(())
}
