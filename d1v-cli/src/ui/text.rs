use crossterm::event::{self, Event, KeyCode, KeyEventKind};

use super::input::InputState;
use super::{is_cancel, Terminal};
use crate::error::Error;

/// Single-line text prompt with optional validation.
pub struct Text {
    label: String,
    validator: Option<Box<dyn Fn(&str) -> Result<(), String>>>,
}

impl Text {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            validator: None,
        }
    }

    pub fn with_validator(mut self, f: impl Fn(&str) -> Result<(), String> + 'static) -> Self {
        self.validator = Some(Box::new(f));
        self
    }

    /// Runs the prompt and returns the entered text.
    pub fn prompt(self) -> Result<String, Error> {
        let mut input = InputState::new();
        let mut error: Option<String> = None;
        let mut term = Terminal::new(1)?;

        loop {
            let height = if error.is_some() { 2 } else { 1 };
            term.set_viewport_height(height)?;

            loop {
                term.draw_prompt(
                    &self.label,
                    input.text(),
                    input.cursor_col(),
                    error.as_deref(),
                    None,
                )?;

                if let Event::Key(key) = event::read()?
                    && key.kind == KeyEventKind::Press
                {
                    match key.code {
                        KeyCode::Enter => break,
                        _ if is_cancel(&key) => {
                            term.show_cancelled(&self.label);
                            return Err(Error::Cancelled);
                        }
                        _ => input.handle_key(&key),
                    }
                }
            }

            if let Some(ref validator) = self.validator
                && let Err(msg) = validator(input.text())
            {
                error = Some(msg);
                continue;
            }

            term.show_answered(&self.label, input.text());
            return Ok(String::from(input));
        }
    }
}
