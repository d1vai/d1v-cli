use anyhow::Result;
use d1v_api::{Code, Email, Validate};
use inquire::Text;

use crate::t;

/// Prompts for an email address with format validation.
pub fn email() -> Result<String> {
    let email = Text::new(&t!("auth-email-prompt"))
        .with_validator(|input: &str| {
            if Email(input).validate().is_ok() {
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
            if Code(input).validate().is_ok() {
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
