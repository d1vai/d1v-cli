use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};
use unicode_width::UnicodeWidthStr;

use super::input::InputState;
use super::{clear_wide_char_continuations, TerminalGuard};
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
            let mut guard = TerminalGuard::new(height)?;

            loop {
                Self::draw(&mut guard, &input, &self.label, error.as_deref())?;

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

            if let Some(ref validator) = self.validator
                && let Err(msg) = validator(input.text())
            {
                error = Some(msg);
                continue;
            }

            // Show answered state
            let text = input.text().to_owned();
            guard.terminal.insert_before(1, |buf| {
                let line = Line::from(vec![
                    Span::styled(
                        &self.label,
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    Span::styled(&text, Style::default().fg(Color::DarkGray)),
                ]);

                Widget::render(Paragraph::new(line), buf.area, buf);
                clear_wide_char_continuations(buf);
            })?;

            return Ok(String::from(input));
        }
    }

    fn draw(
        guard: &mut TerminalGuard,
        input: &InputState,
        label: &str,
        error: Option<&str>,
    ) -> Result<(), std::io::Error> {
        let col = input.cursor_col();
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
                Span::raw(" "),
                Span::raw(input.text()),
            ]));

            frame.render_widget(Paragraph::new(lines), area);

            let error_offset = if error.is_some() { 1 } else { 0 };
            frame.set_cursor_position((label_width + 1 + col as u16, area.y + error_offset));
        })?;

        Ok(())
    }
}
