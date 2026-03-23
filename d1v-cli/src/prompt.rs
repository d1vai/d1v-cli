use anyhow::Result;
use inquire::Text;

use crate::t;

/// Prompts for an email address with format validation.
pub fn email() -> Result<String> {
    let email = Text::new(&t!("auth-email-prompt"))
        .with_validator(|input: &str| {
            let valid = input
                .split_once('@')
                .is_some_and(|(user, domain)| !user.is_empty() && domain.contains('.'));

            if valid {
                Ok(inquire::validator::Validation::Valid)
            } else {
                Ok(inquire::validator::Validation::Invalid(
                    t!("auth-email-invalid").into(),
                ))
            }
        })
        .prompt()?;

    Ok(email)
}

/// Prompts for a 6-digit verification code with format validation.
pub fn code() -> Result<String> {
    let code = Text::new(&t!("auth-code-prompt"))
        .with_validator(|input: &str| {
            if input.len() == 6 && input.chars().all(|c| c.is_ascii_digit()) {
                Ok(inquire::validator::Validation::Valid)
            } else {
                Ok(inquire::validator::Validation::Invalid(
                    t!("auth-code-invalid").into(),
                ))
            }
        })
        .prompt()?;

    Ok(code)
}
