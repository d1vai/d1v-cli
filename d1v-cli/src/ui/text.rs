use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};

use super::input::InputState;
use super::Terminal;
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

        loop {
            let height = if error.is_some() { 2 } else { 1 };
            let mut term = Terminal::new(height)?;

            loop {
                term.draw_prompt(
                    &self.label,
                    input.text(),
                    input.cursor_col(),
                    error.as_deref(),
                )?;

                if let Event::Key(key) = event::read()?
                    && key.kind == KeyEventKind::Press
                {
                    match key.code {
                        KeyCode::Enter => break,
                        KeyCode::Esc | KeyCode::Char('c')
                            if key.code == KeyCode::Esc
                                || key.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
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
