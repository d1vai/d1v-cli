pub mod input;
pub mod password;
pub mod text;

pub use password::Password;
pub use text::Text;

use std::io::{self, Stdout};

use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::buffer::Buffer;
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
