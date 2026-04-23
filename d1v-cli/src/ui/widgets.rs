//! Custom prompt widgets.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::symbols;
use crate::theme;

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
