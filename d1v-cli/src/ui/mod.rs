pub mod input;
pub mod password;
pub mod text;

pub use password::Password;
pub use text::Text;

use std::io::{self, Stdout};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};
use ratatui::{backend::CrosstermBackend, TerminalOptions, Viewport};
use tracing::debug;
use unicode_width::UnicodeWidthStr;

/// Inline terminal for interactive prompt rendering.
///
/// Wraps a ratatui inline-viewport terminal with raw mode management.
/// Entering raw mode on creation and restoring normal mode on drop.
pub struct Terminal {
    inner: ratatui::Terminal<CrosstermBackend<Stdout>>,
    height: u16,
}

impl Terminal {
    /// Enters raw mode and creates an inline terminal with the specified height.
    pub fn new(height: u16) -> io::Result<Self> {
        enable_raw_mode()?;

        let inner = ratatui::Terminal::with_options(
            CrosstermBackend::new(io::stdout()),
            TerminalOptions {
                viewport: Viewport::Inline(height),
            },
        )
        .inspect_err(|_| {
            let _ = disable_raw_mode().inspect_err(|e| debug!("failed to disable raw mode: {e}"));
        })?;

        Ok(Self { inner, height })
    }

    /// Recreates the inline viewport if the height has changed.
    ///
    /// This keeps the Terminal alive across validation retries, avoiding
    /// orphaned viewport content and unnecessary raw mode toggling.
    fn set_viewport_height(&mut self, height: u16) -> io::Result<()> {
        if height != self.height {
            self.inner = ratatui::Terminal::with_options(
                CrosstermBackend::new(io::stdout()),
                TerminalOptions {
                    viewport: Viewport::Inline(height),
                },
            )?;
            self.height = height;
            // The new terminal's previous buffer is all spaces. Clearing
            // the viewport ensures the physical screen matches, preventing
            // stale characters left over from the prior terminal instance.
            self.inner.clear()?;
        }
        Ok(())
    }

    /// Draws the active prompt with cursor, and an optional error message above it.
    fn draw_prompt(
        &mut self,
        label: impl AsRef<str>,
        input_text: impl AsRef<str>,
        cursor_col: usize,
        error: Option<&str>,
    ) -> io::Result<()> {
        let label = label.as_ref();
        let input_text = input_text.as_ref();
        let label_width = label.width() as u16;

        self.inner.draw(|frame| {
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
                Span::raw(input_text),
            ]));

            frame.render_widget(Paragraph::new(lines), area);

            let error_offset = if error.is_some() { 1 } else { 0 };
            frame.set_cursor_position((label_width + 1 + cursor_col as u16, area.y + error_offset));
        })?;

        Ok(())
    }

    /// Renders the answered state above the inline viewport and terminates it.
    fn show_answered(&mut self, label: impl AsRef<str>, display: impl AsRef<str>) {
        let label = label.as_ref();
        let display = display.as_ref();

        let _ = self
            .inner
            .insert_before(1, |buf| {
                let line = Line::from(vec![
                    Span::styled(
                        label,
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    Span::styled(display, Style::default().fg(Color::DarkGray)),
                ]);

                Widget::render(Paragraph::new(line), buf.area, buf);
                clear_wide_char_continuations(buf);
            })
            .inspect_err(|err| debug!("failed to render answered state: {err}"));
    }

    /// Renders the canceled prompt state, showing only the label in gray.
    fn show_cancelled(&mut self, label: impl AsRef<str>) {
        let label = label.as_ref();

        let _ = self
            .inner
            .insert_before(1, |buf| {
                let line = Line::from(Span::styled(
                    label,
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                ));
                Widget::render(Paragraph::new(line), buf.area, buf);
                clear_wide_char_continuations(buf);
            })
            .inspect_err(|err| debug!("failed to render cancelled state: {err}"));
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = disable_raw_mode().inspect_err(|e| debug!("failed to disable raw mode: {e}"));
    }
}

/// Checks if a key event is a cancel shortcut (Esc or Ctrl+C).
fn is_cancel(key: &KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => true,
        KeyCode::Char('c') => key.modifiers.contains(KeyModifiers::CONTROL),
        _ => false,
    }
}

/// Clears continuation cells of wide characters in the buffer.
///
/// Works around a ratatui bug where [`ratatui::Terminal::insert_before`] renders
/// visible spaces for continuation cells of wide characters (CJK, emoji).
/// Setting their symbol to `""` makes the backend's `Print("")` a no-op.
///
/// <https://github.com/ratatui/ratatui/issues/1332>
fn clear_wide_char_continuations(buf: &mut Buffer) {
    let width = buf.area.width as usize;
    for row in 0..buf.area.height {
        let mut col = 0;
        // Walk through each cell, advancing by the cell's display width.
        // For wide characters (width > 1), the trailing "continuation" cells
        // hold Cell::EMPTY (symbol = " "). draw_lines sends these to the
        // backend as Print(" "), producing visible spurious spaces.
        // Setting symbol to "" makes Print("") a no-op, suppressing them.
        while col < width {
            let cell_width = buf[(col as u16, row)].symbol().width().max(1);
            for c in (col + 1)..(col + cell_width).min(width) {
                buf[(c as u16, row)].set_symbol("");
            }
            col += cell_width;
        }
    }
}
