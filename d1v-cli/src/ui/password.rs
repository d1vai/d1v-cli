use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use secrecy::{ExposeSecret, SecretString};
use std::iter;

use super::input::InputState;
use super::Terminal;
use crate::error::Error;
use crate::t;

const MASK: char = '•';

/// Masked password prompt with optional confirmation.
pub struct Password {
    label: String,
    confirmation: Option<String>,
}

impl Password {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            confirmation: None,
        }
    }

    pub fn with_confirmation(mut self, label: impl Into<String>) -> Self {
        self.confirmation = Some(label.into());
        self
    }

    /// Runs the prompt and returns the entered password.
    pub fn prompt(self) -> Result<SecretString, Error> {
        let mut error: Option<String> = None;

        loop {
            let first = self.read(&self.label, error.take())?;

            let Some(confirm_label) = &self.confirmation else {
                return Ok(first);
            };

            match self.read(confirm_label, None) {
                Ok(second) if second.expose_secret() == first.expose_secret() => return Ok(first),
                Ok(_) => error = Some(t!("password-mismatch")),
                Err(err) if err.is_cancelled() => continue,
                Err(err) => return Err(err),
            }
        }
    }

    fn read(&self, label: &str, error: Option<String>) -> Result<SecretString, Error> {
        let height = if error.is_some() { 2 } else { 1 };
        let mut term = Terminal::new(height)?;
        let mut input = InputState::new();

        loop {
            let masked = input.masked(MASK);
            let col = input.masked_cursor_col(MASK);
            term.draw_prompt(label, &masked, col, error.as_deref())?;

            if let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                match key.code {
                    KeyCode::Enter => break,
                    KeyCode::Esc | KeyCode::Char('c')
                        if key.code == KeyCode::Esc
                            || key.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        term.show_cancelled(label);
                        return Err(Error::Cancelled);
                    }
                    _ => input.handle_key(&key),
                }
            }
        }

        let count = input.grapheme_count();
        let answered = iter::repeat(MASK).take(count).collect::<String>();
        term.show_answered(label, &answered)?;

        Ok(SecretString::from(String::from(input)))
    }
}
