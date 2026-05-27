use std::io::{self, IsTerminal};

use anyhow::anyhow;

use crate::ui::{Select, SelectOption};
use crate::{Context, Result, t};

use super::ApiKeyRevokeArgs;

pub async fn run(ctx: &Context, args: ApiKeyRevokeArgs) -> Result<()> {
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

    if !args.yes {
        if io::stdin().is_terminal() {
            let confirmed = confirm_revoke(&key.name, &key.key_prefix)?;
            if !confirmed {
                return Ok(());
            }
        } else {
            return Err(anyhow!("{}", t!("api-key-revoke-confirm-required")).into());
        }
    }

    ctx.client.user().revoke_api_key(key.id).await?;
    ctx.success(t!("api-key-revoke-success", name = &key.name));
    Ok(())
}

fn confirm_revoke(name: &str, prefix: &str) -> Result<bool> {
    enum Choice {
        Confirm,
        Cancel,
    }

    let choice = Select::new(t!(
        "api-key-revoke-confirm-prompt",
        name = name,
        prefix = prefix
    ))
    .option(SelectOption::new(
        Choice::Confirm,
        t!("api-key-revoke-confirm-yes"),
    ))
    .option(SelectOption::new(
        Choice::Cancel,
        t!("api-key-revoke-confirm-no"),
    ))
    .default_index(1)
    .prompt()?;

    Ok(matches!(choice, Choice::Confirm))
}
