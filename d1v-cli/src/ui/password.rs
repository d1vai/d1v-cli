use secrecy::{ExposeSecret, SecretString};
use std::iter;

use super::input::InputState;
use super::Terminal;
use crate::error::Error;
use crate::t;

const MASK: char = '•';

/// Masked password prompt with optional confirmation and validation.
pub struct Password {
    label: String,
    confirmation: Option<String>,
    validator: Option<Box<dyn Fn(&str) -> Result<(), String>>>,
}

impl Password {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            confirmation: None,
            validator: None,
        }
    }

    pub fn with_confirmation(mut self, label: impl Into<String>) -> Self {
        self.confirmation = Some(label.into());
        self
    }

    pub fn with_validator(mut self, f: impl Fn(&str) -> Result<(), String> + 'static) -> Self {
        self.validator = Some(Box::new(f));
        self
    }

    /// Runs the prompt and returns the entered password.
    pub fn prompt(self) -> Result<SecretString, Error> {
        let mut error: Option<String> = None;

        loop {
            let first = self.read(&self.label, error.take(), self.validator.as_deref())?;

            let Some(confirm_label) = &self.confirmation else {
                return Ok(first);
            };

            match self.read(confirm_label, None, None) {
                Ok(second) if second.expose_secret() == first.expose_secret() => return Ok(first),
                Ok(_) => error = Some(t!("password-mismatch")),
                Err(err) if err.is_cancelled() => continue,
                Err(err) => return Err(err),
            }
        }
    }

    fn read(
        &self,
        label: &str,
        error: Option<String>,
        validator: Option<&dyn Fn(&str) -> Result<(), String>>,
    ) -> Result<SecretString, Error> {
        let mut error = error;
        let mut term = Terminal::new(if error.is_some() { 2 } else { 1 })?;

        loop {
            let height = if error.is_some() { 2 } else { 1 };
            term.set_viewport_height(height)?;
            let mut input = InputState::new();

            loop {
                let masked = input.masked(MASK);
                let col = input.masked_cursor_col(MASK);
                term.draw_prompt(label, &masked, col, error.as_deref(), None)?;

                if term.read_key(&mut input, label)? {
                    break;
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
            term.show_answered(label, &answered);

            return Ok(SecretString::from(String::from(input)));
        }
    }
}
