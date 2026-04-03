use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use secrecy::{ExposeSecret, SecretString};
use unicode_width::UnicodeWidthStr;

use super::input::InputState;
use super::TerminalGuard;
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
    pub fn prompt(self) -> Result<SecretString, anyhow::Error> {
        loop {
            let first = self.read(&self.label)?;

            let Some(label) = &self.confirmation else {
                break Ok(first);
            };

            match self.read(label) {
                Ok(second) if second.expose_secret() == first.expose_secret() => return Ok(first),
                Ok(_) => eprintln!("{}", t!("password-mismatch")),
                Err(e)
                    if e.downcast_ref::<Error>()
                        .is_some_and(|e| matches!(e, Error::Cancelled)) =>
                {
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }

    fn read(&self, label: &str) -> Result<SecretString, anyhow::Error> {
        let mut guard = TerminalGuard::new(1)?;
        let mut input = InputState::new();

        loop {
            Self::draw(&mut guard, &input, label)?;

            if let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                match key.code {
                    KeyCode::Enter => break,
                    KeyCode::Esc => return Err(Error::Cancelled.into()),
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Err(Error::Cancelled.into());
                    }
                    _ => input.handle_key(&key),
                }
            }
        }

        guard.terminal.insert_before(1, |_| {})?;

        Ok(SecretString::from({
            let s: String = input.into();
            s
        }))
    }

    fn draw(
        guard: &mut TerminalGuard,
        input: &InputState,
        label: &str,
    ) -> Result<(), std::io::Error> {
        let masked = input.masked(MASK);
        let col = input.masked_cursor_col(MASK);
        let label_width = label.width() as u16;

        guard.terminal.draw(|frame| {
            let area = frame.area();
            let line = Line::from(vec![
                Span::styled(
                    label,
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(&masked),
            ]);

            frame.render_widget(Paragraph::new(line), area);
            frame.set_cursor_position((label_width + col as u16, area.y));
        })?;

        Ok(())
    }
}
