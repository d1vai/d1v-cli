use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};
use secrecy::{ExposeSecret, SecretString};
use std::iter;
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
        let mut guard = TerminalGuard::new(height)?;
        let mut input = InputState::new();

        loop {
            Self::draw(&mut guard, &input, label, error.as_deref())?;

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

        // Show answered state
        let count = input.grapheme_count();
        guard.terminal.insert_before(1, |buf| {
            let answered = iter::repeat(MASK).take(count).collect::<String>();

            let line = Line::from(vec![
                Span::styled(
                    label,
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(answered, Style::default().fg(Color::DarkGray)),
            ]);

            Widget::render(Paragraph::new(line), buf.area, buf);
        })?;

        Ok(SecretString::from(String::from(input)))
    }

    fn draw(
        guard: &mut TerminalGuard,
        input: &InputState,
        label: &str,
        error: Option<&str>,
    ) -> Result<(), std::io::Error> {
        let masked = input.masked(MASK);
        let col = input.masked_cursor_col(MASK);
        let label_width = label.width() as u16;

        guard.terminal.draw(|frame| {
            let area = frame.area();
            let mut lines = Vec::new();

            if let Some(msg) = error {
                lines.push(Line::from(Span::styled(
                    msg,
                    Style::default().fg(Color::Red),
                )));
            }

            lines.push(Line::from(vec![
                Span::styled(
                    label,
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(&masked),
            ]));

            frame.render_widget(Paragraph::new(lines), area);

            let error_offset = if error.is_some() { 1 } else { 0 };
            frame.set_cursor_position((label_width + col as u16, area.y + error_offset));
        })?;

        Ok(())
    }
}
