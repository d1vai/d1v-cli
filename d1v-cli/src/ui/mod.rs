pub mod confirm;
pub mod input;
pub mod keys;
pub mod password;
pub mod prompt;
pub mod select;
pub mod text;
pub mod widgets;

pub use confirm::Confirm;
pub use password::Password;
pub use prompt::PendingPrompt;
pub use select::{Select, SelectOption};
pub use text::Text;
pub use widgets::{Answered, Canceled, Inline, Pending, Prompt, SelectItem, SelectList, Toggle};

use crossterm::cursor::MoveTo;
use crossterm::event::{self, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use crossterm::QueueableCommand;
use ratatui::buffer::Buffer;
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;
use ratatui::{backend::CrosstermBackend, TerminalOptions, Viewport};
use std::io::{self, Stdout, Write};
use tracing::debug;
use unicode_width::UnicodeWidthStr;

use crate::t;
use crate::theme;

/// Fixed display-width of the prompt status prefix (`◆ `, `✓ `, `✗ `).
const PREFIX_WIDTH: u16 = 2;

pub type Validator = dyn Fn(&str) -> Result<(), String>;

/// Casts a `usize` to a `u16`, saturating on overflow.
#[inline]
fn as_u16(n: usize) -> u16 {
    u16::try_from(n).unwrap_or(u16::MAX)
}

/// Inline terminal for interactive prompt rendering.
///
/// Wraps a ratatui inline-viewport terminal. Enters raw mode on creation
/// and restores it on drop.
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

    /// Adjusts the viewport height, clearing stale content beforehand.
    fn set_viewport_height(&mut self, height: u16) -> io::Result<()> {
        if height != self.height {
            self.inner.clear()?;
            self.inner = ratatui::Terminal::with_options(
                CrosstermBackend::new(io::stdout()),
                TerminalOptions {
                    viewport: Viewport::Inline(height),
                },
            )?;
            self.height = height;
        }
        Ok(())
    }

    fn insert_widget_before(&mut self, height: u16, widget: impl Widget) -> io::Result<()> {
        self.set_viewport_height(height)?;
        self.inner.insert_before(height, |buf| {
            widget.render(buf.area, buf);
            clear_wide_char_continuations(buf);
        })
    }

    /// Renders an inline `widget`, sizing the viewport to its desired height.
    pub fn render<W>(&mut self, widget: &W) -> io::Result<()>
    where
        W: Inline,
        for<'a> &'a W: Widget,
    {
        self.set_viewport_height(widget.height())?;
        self.inner.draw(|frame| {
            let area = frame.area();
            frame.render_widget(widget, area);

            // ratatui's `Terminal::draw` hides the cursor when the closure does
            // not call `Frame::set_cursor_position`, and shows it otherwise.
            if let Some(pos) = widget.cursor_position(area) {
                frame.set_cursor_position(pos);
            }
        })?;
        Ok(())
    }

    /// Commits a final `widget` above the inline viewport.
    pub fn commit<W>(&mut self, widget: &W)
    where
        W: Inline,
        for<'a> &'a W: Widget,
    {
        if let Err(err) = self.insert_widget_before(widget.height(), widget) {
            debug!("failed to commit inline widget: {err}");
        }
    }

    /// Renders a final `widget` and parks the cursor below the viewport.
    ///
    /// Unlike [`Self::commit`], this keeps the frame in the inline viewport.
    pub fn finish<W>(&mut self, widget: &W) -> io::Result<()>
    where
        W: Inline,
        for<'a> &'a W: Widget,
    {
        self.set_viewport_height(widget.height())?;
        let area = self.inner.get_frame().area();
        self.inner.draw(|frame| frame.render_widget(widget, area))?;

        // Move to the last viewport row and emit `\n`.
        let last_row = area.bottom().saturating_sub(1);
        let mut out = io::stdout();
        out.queue(MoveTo(0, last_row))?;
        out.write_all(b"\n")?;
        out.flush()
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = disable_raw_mode().inspect_err(|e| debug!("failed to disable raw mode: {e}"));
    }
}

pub fn nav_hint_line() -> Line<'static> {
    let sep = || Span::styled(" · ", theme::tui::dim());
    let key = |s: &'static str| Span::styled(s, theme::tui::key());
    let act = |s: String| Span::styled(s, theme::tui::dim());

    Line::from(vec![
        Span::raw("  "),
        key("↑↓"),
        Span::raw(" "),
        act(t!("select-action-navigate")),
        sep(),
        key("Enter"),
        Span::raw(" "),
        act(t!("select-action-confirm")),
        sep(),
        key("Esc"),
        Span::raw(" "),
        act(t!("select-action-cancel")),
    ])
}

pub fn ctrl_c_hint_line() -> Line<'static> {
    let key = keys::ctrl_c_label();
    let rendered = t!("select-ctrl-c-hint", key = key);

    let mut spans = vec![Span::raw("  ")];

    if let Some((before, after)) = rendered.split_once(key) {
        if !before.is_empty() {
            spans.push(Span::styled(before.to_string(), theme::tui::dim()));
        }

        spans.push(Span::styled(key, theme::tui::key()));

        if !after.is_empty() {
            spans.push(Span::styled(after.to_string(), theme::tui::dim()));
        }
    } else {
        spans.push(Span::styled(rendered, theme::tui::dim()))
    }

    Line::from(spans)
}

/// Key press classified as a prompt action.
enum Action {
    /// Submit current input (Enter).
    Submit,
    /// Cancel the prompt (Esc / Ctrl+C).
    Cancel,
    /// Forward to input handling.
    Input(KeyEvent),
}

impl Action {
    /// Reads one key event and classifies it.
    ///
    /// Returns `None` for non-key-press events (release, repeat).
    fn read() -> io::Result<Option<Self>> {
        let Some(key) = event::read()?.as_key_press_event() else {
            return Ok(None);
        };

        Ok(Some(match key.code {
            KeyCode::Enter => Action::Submit,
            KeyCode::Esc => Action::Cancel,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Cancel,
            _ => Action::Input(key),
        }))
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
            let cell_width = buf[(as_u16(col), row)].symbol().width().max(1);
            for c in (col + 1)..(col + cell_width).min(width) {
                buf[(as_u16(c), row)].set_symbol("");
            }
            col += cell_width;
        }
    }
}
