use anyhow::Result;
use d1v_api::{Code, Email, Validate};
use inquire::validator::{ErrorMessage, Validation};
use inquire::Text;

use crate::t;

macro_rules! validator {
    ($type:ident, $msg:expr) => {
        |input: &str| -> Result<Validation, inquire::CustomUserError> {
            Ok(if $type(input).validate().is_ok() {
                Validation::Valid
            } else {
                Validation::Invalid(ErrorMessage::Custom($msg))
            })
        }
    };
}

/// Prompts for an email address with format validation.
pub fn email() -> Result<String> {
    Ok(Text::new(&t!("auth-email-prompt"))
        .with_validator(validator!(Email, t!("auth-email-invalid")))
        .prompt()?)
}

/// Prompts for a 6-digit verification code with format validation.
pub fn code() -> Result<String> {
    Ok(Text::new(&t!("auth-code-prompt"))
        .with_validator(validator!(Code, t!("auth-code-invalid")))
        .prompt()?)
}
