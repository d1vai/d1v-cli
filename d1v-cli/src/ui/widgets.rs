//! Custom prompt widgets.

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};
use unicode_width::UnicodeWidthStr;

use super::{as_u16, PREFIX_WIDTH};
use crate::symbols;
use crate::theme;

/// A widget rendered into the inline viewport.
pub trait Inline {
    /// Number of rows this widget needs.
    fn height(&self) -> u16;

    /// Cursor position within `area`, or `None` to hide the cursor.
    fn cursor_position(&self, _area: Rect) -> Option<(u16, u16)> {
        None
    }
}

pub struct Prompt<'a> {
    label: &'a str,
    input: &'a str,
    cursor_col: usize,
    error: Option<&'a str>,
    help: Option<&'a str>,
}

impl<'a> Prompt<'a> {
    pub const fn new(label: &'a str, input: &'a str, cursor_col: usize) -> Self {
        Self {
            label,
            input,
            cursor_col,
            error: None,
            help: None,
        }
    }

    #[must_use]
    pub const fn error(mut self, msg: &'a str) -> Self {
        self.error = Some(msg);
        self
    }

    #[must_use]
    pub const fn help(mut self, msg: &'a str) -> Self {
        self.help = Some(msg);
        self
    }
}

impl Inline for Prompt<'_> {
    fn height(&self) -> u16 {
        1 + u16::from(self.error.is_some()) + u16::from(self.help.is_some())
    }

    fn cursor_position(&self, area: Rect) -> Option<(u16, u16)> {
        let label_width = as_u16(self.label.width());
        let error_offset = u16::from(self.error.is_some());
        Some((
            area.x + PREFIX_WIDTH + label_width + 1 + as_u16(self.cursor_col),
            area.y + error_offset,
        ))
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
    label: &'a str,
    display: &'a str,
}

impl<'a> Answered<'a> {
    pub const fn new(label: &'a str, display: &'a str) -> Self {
        Self { label, display }
    }
}

impl Inline for Answered<'_> {
    fn height(&self) -> u16 {
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

pub struct Toggle<'a> {
    label: &'a str,
    options: [&'a str; 2],
    selected: usize,
}

impl<'a> Toggle<'a> {
    pub const fn new(label: &'a str, options: [&'a str; 2]) -> Self {
        Self {
            label,
            options,
            selected: 0,
        }
    }

    #[must_use]
    pub const fn selected(mut self, selected: usize) -> Self {
        self.selected = selected;
        self
    }
}

impl Inline for Toggle<'_> {
    fn height(&self) -> u16 {
        1
    }
}

impl Widget for &Toggle<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let styles = if self.selected == 0 {
            [theme::tui::active(), theme::tui::dim()]
        } else {
            [theme::tui::dim(), theme::tui::active()]
        };

        let line = Line::from(vec![
            Span::styled(symbols::PROMPT_PREFIX, theme::tui::prompt()),
            Span::styled(self.label, theme::tui::label()),
            Span::raw("  "),
            Span::styled(self.options[0], styles[0]),
            Span::styled(" / ", theme::tui::dim()),
            Span::styled(self.options[1], styles[1]),
        ]);
        Paragraph::new(line).render(area, buf);
    }
}

pub struct SelectItem<'a> {
    pub label: &'a str,
    pub description: Option<&'a str>,
}

struct SelectRow<'a> {
    item: &'a SelectItem<'a>,
    index: usize,
    num_width: usize,
    max_label_w: usize,
    active: bool,
}

impl<'a> SelectRow<'a> {
    const fn new(
        item: &'a SelectItem<'a>,
        index: usize,
        num_width: usize,
        max_label_w: usize,
    ) -> Self {
        Self {
            item,
            index,
            num_width,
            max_label_w,
            active: false,
        }
    }

    #[must_use]
    const fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }
}

impl Widget for &SelectRow<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let num = format!(
            "{:>width$}",
            format!("{}.", self.index + 1),
            width = self.num_width
        );

        let mut spans = vec![
            if self.active {
                Span::styled(format!(" {} ", symbols::SELECT_ARROW), theme::tui::prompt())
            } else {
                Span::raw("   ")
            },
            Span::styled(num, theme::tui::dim()),
            Span::raw(" "),
            Span::styled(
                self.item.label,
                if self.active {
                    theme::tui::value()
                } else {
                    theme::tui::inactive()
                },
            ),
        ];

        if let Some(desc) = self.item.description {
            let pad = " ".repeat(self.max_label_w.saturating_sub(self.item.label.width()) + 3);
            spans.push(Span::raw(pad));
            spans.push(Span::styled(desc, theme::tui::dim()));
        }

        Line::from(spans).render(area, buf);
    }
}

pub struct SelectList<'a> {
    label: &'a str,
    description: Option<&'a str>,
    items: &'a [SelectItem<'a>],
    selected: usize,
    hint: Line<'a>,
}

impl<'a> SelectList<'a> {
    pub fn new(label: &'a str, items: &'a [SelectItem<'a>]) -> Self {
        Self {
            label,
            description: None,
            items,
            selected: 0,
            hint: Line::default(),
        }
    }

    #[must_use]
    pub const fn description(mut self, desc: &'a str) -> Self {
        self.description = Some(desc);
        self
    }

    #[must_use]
    pub const fn selected(mut self, selected: usize) -> Self {
        self.selected = selected;
        self
    }

    #[must_use]
    pub fn hint(mut self, hint: Line<'a>) -> Self {
        self.hint = hint;
        self
    }
}

impl Inline for SelectList<'_> {
    fn height(&self) -> u16 {
        as_u16(self.items.len()) + 6 + if self.description.is_some() { 2 } else { 0 }
    }
}

impl Widget for &SelectList<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let n = self.items.len();
        let num_width = format!("{n}.").len();
        let max_label_w: usize = self
            .items
            .iter()
            .map(|i| i.label.width())
            .max()
            .unwrap_or(0);

        let desc_rows: u16 = if self.description.is_some() { 2 } else { 0 };
        let chunks = Layout::vertical([
            Constraint::Length(1),         // blank
            Constraint::Length(1),         // header
            Constraint::Length(desc_rows), // optional [blank, description]
            Constraint::Length(1),         // blank
            Constraint::Length(as_u16(n)), // items
            Constraint::Length(1),         // blank
            Constraint::Length(1),         // hint
            Constraint::Length(1),         // blank
        ])
        .split(area);

        Line::from(vec![
            Span::styled(symbols::SELECT_PREFIX, theme::tui::prompt()),
            Span::styled(self.label, theme::tui::label()),
        ])
        .render(chunks[1], buf);

        if let Some(desc) = self.description {
            let desc_chunks =
                Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(chunks[2]);
            Line::from(Span::styled(
                format!("   {desc}"),
                theme::tui::description(),
            ))
            .render(desc_chunks[1], buf);
        }

        for (i, (item, row)) in self.items.iter().zip(chunks[4].rows()).enumerate() {
            SelectRow::new(item, i, num_width, max_label_w)
                .active(i == self.selected)
                .render(row, buf);
        }

        self.hint.clone().render(chunks[6], buf);
    }
}

pub struct Pending<'a> {
    label: &'a str,
    display: &'a str,
    spinner: &'a str,
}

impl<'a> Pending<'a> {
    pub const fn new(label: &'a str, display: &'a str, spinner: &'a str) -> Self {
        Self {
            label,
            display,
            spinner,
        }
    }
}

impl Inline for Pending<'_> {
    fn height(&self) -> u16 {
        1
    }
}

impl Widget for &Pending<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let line = Line::from(vec![
            Span::styled(format!("{} ", self.spinner), theme::tui::prompt()),
            Span::styled(self.label, theme::tui::label()),
            Span::raw(" "),
            Span::styled(self.display, theme::tui::value()),
        ]);
        Paragraph::new(line).render(area, buf);
    }
}

pub struct Canceled<'a> {
    label: &'a str,
    display: &'a str,
}

impl<'a> Canceled<'a> {
    pub const fn new(label: &'a str, display: &'a str) -> Self {
        Self { label, display }
    }
}

impl Inline for Canceled<'_> {
    fn height(&self) -> u16 {
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
