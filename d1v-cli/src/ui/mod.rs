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
pub use widgets::{Answered, Canceled, Inline, Prompt};

use crossterm::event::{self, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::buffer::Buffer;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};
use ratatui::{backend::CrosstermBackend, TerminalOptions, Viewport};
use std::io::{self, Stdout};
use tracing::debug;
use unicode_width::UnicodeWidthStr;

use crate::symbols;
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
    pub fn commit<W>(&mut self, widget: &W) -> io::Result<()>
    where
        W: Inline,
        for<'a> &'a W: Widget,
    {
        self.insert_widget_before(widget.height(), widget)
    }

    /// Draws the prompt line with cursor, optional error above and help below.
    fn draw_prompt(
        &mut self,
        label: impl AsRef<str>,
        input_text: impl AsRef<str>,
        cursor_col: usize,
        error: Option<&str>,
        help: Option<&str>,
    ) -> io::Result<()> {
        let mut prompt = Prompt::new(label.as_ref(), input_text.as_ref(), cursor_col);
        if let Some(msg) = error {
            prompt = prompt.error(msg);
        }
        if let Some(msg) = help {
            prompt = prompt.help(msg);
        }
        self.render(&prompt)
    }

    /// Draws an inline toggle selector.
    fn draw_toggle(
        &mut self,
        label: impl AsRef<str>,
        options: [&str; 2],
        selected: usize,
    ) -> io::Result<()> {
        let label = label.as_ref();

        let styles = if selected == 0 {
            [theme::tui::active(), theme::tui::dim()]
        } else {
            [theme::tui::dim(), theme::tui::active()]
        };

        self.inner.hide_cursor()?;
        self.inner.draw(|frame| {
            let line = Line::from(vec![
                Span::styled(symbols::PROMPT_PREFIX, theme::tui::prompt()),
                Span::styled(label, theme::tui::label()),
                Span::raw("  "),
                Span::styled(options[0], styles[0]),
                Span::styled(" / ", theme::tui::dim()),
                Span::styled(options[1], styles[1]),
            ]);
            frame.render_widget(Paragraph::new(line), frame.area());
        })?;

        Ok(())
    }

    /// Renders the answered state above the inline viewport and terminates it.
    fn show_answered(&mut self, label: impl AsRef<str>, display: impl AsRef<str>) {
        let _ = self
            .commit(&Answered::new(label.as_ref(), display.as_ref()))
            .inspect_err(|err| debug!("failed to render answered state: {err}"));
    }

    /// Renders the canceled prompt state with the label and partial input.
    fn show_canceled(&mut self, label: impl AsRef<str>, display: impl AsRef<str>) {
        let _ = self
            .commit(&Canceled::new(label.as_ref(), display.as_ref()))
            .inspect_err(|err| debug!("failed to render canceled state: {err}"));
    }

    /// Draws the spinner state on the viewport without committing.
    fn show_pending(
        &mut self,
        label: impl AsRef<str>,
        display: impl AsRef<str>,
        spinner: impl AsRef<str>,
    ) {
        let label = label.as_ref();
        let display = display.as_ref();
        let spinner = spinner.as_ref();

        let _ = self.inner.hide_cursor();
        let _ = self
            .inner
            .draw(|frame| {
                let line = Line::from(vec![
                    Span::styled(format!("{spinner} "), theme::tui::prompt()),
                    Span::styled(label, theme::tui::label()),
                    Span::raw(" "),
                    Span::styled(display, theme::tui::value()),
                ]);
                Paragraph::new(line).render(frame.area(), frame.buffer_mut());
            })
            .inspect_err(|err| debug!("failed to render pending state: {err}"));
    }

    pub fn draw_select(
        &mut self,
        label: impl AsRef<str>,
        description: Option<&str>,
        items: &[SelectItem<'_>],
        selected: usize,
        hint: Line<'_>,
    ) -> io::Result<()> {
        let label = label.as_ref();
        let n = as_u16(items.len());
        self.set_viewport_height(n + 6 + if description.is_some() { 2 } else { 0 })?;
        self.inner.hide_cursor()?;

        // Width of the widest index string, e.g. "10." for 10 items.
        let num_width = format!("{n}.").len();
        // Width of the longest option label for description column alignment.
        let max_label_w: usize = items.iter().map(|i| i.label.width()).max().unwrap_or(0);

        self.inner.draw(|frame| {
            let mut lines = Vec::with_capacity((n + 6) as usize);

            lines.push(Line::raw(""));

            lines.push(Line::from(vec![
                Span::styled(symbols::SELECT_PREFIX, theme::tui::prompt()),
                Span::styled(label, theme::tui::label()),
            ]));

            if let Some(desc) = description {
                lines.push(Line::raw(""));
                lines.push(Line::from(Span::styled(
                    format!("   {desc}"),
                    theme::tui::description(),
                )));
            }

            lines.push(Line::raw(""));

            for (i, item) in items.iter().enumerate() {
                lines.push(item.render(i, i == selected, num_width, max_label_w));
            }

            lines.push(Line::raw(""));

            lines.push(hint);

            lines.push(Line::raw(""));

            frame.render_widget(Paragraph::new(lines), frame.area());
        })?;

        Ok(())
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

pub struct SelectItem<'a> {
    pub label: &'a str,
    pub description: Option<&'a str>,
}

impl<'a> SelectItem<'a> {
    pub fn render(
        &self,
        index: usize,
        active: bool,
        num_width: usize,
        max_label_w: usize,
    ) -> Line<'a> {
        let num = format!("{:>width$}", format!("{}.", index + 1), width = num_width);

        if active {
            self.render_active(num, max_label_w)
        } else {
            self.render_inactive(num, max_label_w)
        }
    }

    fn render_active(&self, num: String, max_label_w: usize) -> Line<'a> {
        let mut spans = vec![
            Span::raw(" "),
            Span::styled(symbols::SELECT_ARROW, theme::tui::prompt()),
            Span::raw(" "),
            Span::styled(num, theme::tui::dim()),
            Span::raw(" "),
            Span::styled(self.label, theme::tui::value()),
        ];

        if let Some(desc) = self.description {
            let pad = " ".repeat(max_label_w.saturating_sub(self.label.width()) + 3);
            spans.push(Span::raw(pad));
            spans.push(Span::styled(desc, theme::tui::dim()));
        }

        Line::from(spans)
    }

    fn render_inactive(&self, num: String, max_label_w: usize) -> Line<'a> {
        let mut spans = vec![
            Span::raw("   "),
            Span::styled(num, theme::tui::dim()),
            Span::raw(" "),
            Span::styled(self.label, theme::tui::inactive()),
        ];

        if let Some(desc) = self.description {
            let pad = " ".repeat(max_label_w.saturating_sub(self.label.width()) + 3);
            spans.push(Span::raw(pad));
            spans.push(Span::styled(desc, theme::tui::dim()));
        }

        Line::from(spans)
    }
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
