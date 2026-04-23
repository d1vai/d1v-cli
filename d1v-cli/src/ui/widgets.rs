//! Custom prompt widgets.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};
use unicode_width::UnicodeWidthStr;

use super::{as_u16, PREFIX_WIDTH};
use crate::symbols;
use crate::theme;

pub struct Prompt<'a> {
    pub label: &'a str,
    pub input: &'a str,
    pub cursor_col: usize,
    pub error: Option<&'a str>,
    pub help: Option<&'a str>,
}

impl Prompt<'_> {
    pub fn height(&self) -> u16 {
        1 + u16::from(self.error.is_some()) + u16::from(self.help.is_some())
    }

    /// Cursor position relative to the buffer origin, on the prompt line.
    pub fn cursor_position(&self, area: Rect) -> (u16, u16) {
        let label_width = as_u16(self.label.width());
        let error_offset = u16::from(self.error.is_some());
        (
            area.x + PREFIX_WIDTH + label_width + 1 + as_u16(self.cursor_col),
            area.y + error_offset,
        )
    }
}

impl Widget for &Prompt<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut lines = Vec::with_capacity(self.height() as usize);

        if let Some(msg) = self.error {
            lines.push(Line::from(Span::styled(msg, theme::tui::error())));
        }

        lines.push(Line::from(vec![
            Span::styled(symbols::PROMPT_PREFIX, theme::tui::prompt()),
            Span::styled(self.label, theme::tui::label()),
            Span::raw(" "),
            Span::raw(self.input),
        ]));

        if let Some(msg) = self.help {
            lines.push(Line::from(Span::styled(msg, theme::tui::dim())));
        }

        Paragraph::new(lines).render(area, buf);
    }
}

pub struct Answered<'a> {
    pub label: &'a str,
    pub display: &'a str,
}

impl Answered<'_> {
    pub const fn height(&self) -> u16 {
        1
    }
}

impl Widget for &Answered<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let line = Line::from(vec![
            Span::styled(symbols::SUCCESS_PREFIX, theme::tui::success()),
            Span::styled(self.label, theme::tui::label()),
            Span::raw(" "),
            Span::styled(self.display, theme::tui::value()),
        ]);
        Paragraph::new(line).render(area, buf);
    }
}

pub struct Canceled<'a> {
    pub label: &'a str,
    pub display: &'a str,
}

impl Canceled<'_> {
    pub const fn height(&self) -> u16 {
        1
    }
}

impl Widget for &Canceled<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let line = Line::from(vec![
            Span::styled(symbols::ERROR_PREFIX, theme::tui::error()),
            Span::styled(self.label, theme::tui::error()),
            Span::raw(" "),
            Span::styled(self.display, theme::tui::dim()),
        ]);
        Paragraph::new(line).render(area, buf);
    }
}
