use secrecy::{ExposeSecret, SecretString};
use std::iter;

use super::input::InputState;
use super::widgets::{Answered, Canceled, Prompt};
use super::{Action, Terminal, Validator};
use crate::error::Error;
use crate::t;

const MASK: char = '•';

/// Masked password prompt with optional confirmation and validation.
pub struct Password {
    label: String,
    confirmation: Option<String>,
    validator: Option<Box<Validator>>,
}

impl Password {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            confirmation: None,
            validator: None,
        }
    }

    #[must_use]
    pub fn with_confirmation(mut self, label: impl Into<String>) -> Self {
        self.confirmation = Some(label.into());
        self
    }

    #[must_use]
    pub fn with_validator(mut self, f: impl Fn(&str) -> Result<(), String> + 'static) -> Self {
        self.validator = Some(Box::new(f));
        self
    }

    /// Runs the prompt and returns the entered password.
    pub fn prompt(self) -> Result<SecretString, Error> {
        let mut error: Option<String> = None;

        loop {
            let first = read_password(&self.label, error.take(), self.validator.as_deref())?;

            let Some(confirm_label) = &self.confirmation else {
                return Ok(first);
            };

            match read_password(confirm_label, None, None) {
                Ok(second) if second.expose_secret() == first.expose_secret() => return Ok(first),
                Ok(_) => error = Some(t!("password-mismatch")),
                Err(err) if err.is_canceled() => {}
                Err(err) => return Err(err),
            }
        }
    }
}

fn read_password(
    label: &str,
    error: Option<String>,
    validator: Option<&Validator>,
) -> Result<SecretString, Error> {
    let mut error = error;
    let mut term = Terminal::new(if error.is_some() { 2 } else { 1 })?;

    loop {
        let mut input = InputState::new();

        loop {
            let masked = input.masked(MASK);
            let col = input.masked_cursor_col(MASK);

            let mut prompt = Prompt::new(label, &masked, col);
            if let Some(msg) = error.as_deref() {
                prompt = prompt.error(msg);
            }
            term.render(&prompt)?;

            match Action::read()? {
                Some(Action::Submit) => break,
                Some(Action::Cancel) => {
                    term.commit(&Canceled::new(label, &masked));
                    return Err(Error::Canceled);
                }
                Some(Action::Input(key)) => input.handle_key(&key),
                None => {}
            }
        }

        if let Some(validator) = validator
            && let Err(msg) = validator(input.text())
        {
            error = Some(msg);
            continue;
        }

        let count = input.grapheme_count();
        let answered = iter::repeat_n(MASK, count).collect::<String>();
        term.commit(&Answered::new(label, &answered));

        return Ok(SecretString::from(String::from(input)));
    }
}
