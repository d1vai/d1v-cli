pub mod input;
pub mod password;
pub mod text;

pub use password::Password;
pub use text::Text;

use std::io::{self, Stdout};

use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};
use ratatui::{backend::CrosstermBackend, Terminal, TerminalOptions, Viewport};
use unicode_width::UnicodeWidthStr;

pub struct TerminalGuard {
    pub terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalGuard {
    /// Enters raw mode and creates an inline terminal with the specified height.
    pub fn new(height: u16) -> io::Result<Self> {
        enable_raw_mode()?;

        let terminal = Terminal::with_options(
            CrosstermBackend::new(io::stdout()),
            TerminalOptions {
                viewport: Viewport::Inline(height),
            },
        )?;

        Ok(Self { terminal })
    }

    /// Draws the active prompt with cursor, and an optional error message above it.
    fn draw_prompt(
        &mut self,
        label: &str,
        input_text: &str,
        cursor_col: usize,
        error: Option<&str>,
    ) -> io::Result<()> {
        let label_width = label.width() as u16;

        self.terminal.draw(|frame| {
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
    fn show_answered(&mut self, label: &str, display: &str) -> io::Result<()> {
        self.terminal.insert_before(1, |buf| {
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
    }

    /// Renders the canceled prompt state, showing only the label in gray.
    fn show_cancelled(&mut self, label: &str) {
        let _ = self.terminal.insert_before(1, |buf| {
            let line = Line::from(Span::styled(
                label,
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ));
            Widget::render(Paragraph::new(line), buf.area, buf);
            clear_wide_char_continuations(buf);
        });
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

/// Clears continuation cells of wide characters in the buffer.
///
/// Works around a ratatui bug where [`Terminal::insert_before`] renders
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
