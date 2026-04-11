use crate::error::Result;
use crate::localize::Localize;
use d1v_api::{Code, Email, Validate};
use secrecy::SecretString;

use crate::t;
use crate::ui::{Password, Text};

fn validated<E: Localize>(
    f: impl Fn(&str) -> Result<(), E> + 'static,
) -> impl Fn(&str) -> Result<(), String> + 'static {
    move |input| f(input).map_err(|err| err.localize())
}

/// Prompts for an email address with format validation.
pub fn email() -> Result<String> {
    Text::new(t!("auth-email-prompt"))
        .with_validator(validated(|input| Email(input).validate()))
        .prompt()
}

/// Prompts for a 6-digit verification code with format validation.
pub fn code() -> Result<String> {
    Text::new(t!("auth-code-prompt"))
        .with_validator(validated(|input| Code(input).validate()))
        .prompt()
}

/// Prompts for a new password with masked display and confirmation.
pub fn new_password() -> Result<SecretString> {
    Password::new(t!("password-new-prompt"))
        .with_validator(|s| {
            if s.is_empty() {
                Err(t!("password-empty"))
            } else {
                Ok(())
            }
        })
        .with_confirmation(t!("password-confirm-prompt"))
        .prompt()
}
