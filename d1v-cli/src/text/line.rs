use std::borrow::Cow;
use std::fmt::{self, Display, Formatter};
use std::io;

use anstyle::Style;

use super::{Render, RenderContext, Span};

/// Horizontal sequence of [`Span`]s. Renders without trailing newline.
#[derive(Default)]
pub struct Line {
    pub spans: Vec<Span>,
}

impl Line {
    pub fn new() -> Self {
        Line::default()
    }

    pub fn raw(content: impl Into<Cow<'static, str>>) -> Self {
        Self {
            spans: vec![Span::raw(content)],
        }
    }

    pub fn styled(content: impl Into<Cow<'static, str>>, style: Style) -> Self {
        Self {
            spans: vec![Span::styled(content, style)],
        }
    }

    pub fn push_plain(mut self, content: impl Into<Cow<'static, str>>) -> Self {
        self.spans.push(Span::raw(content));
        self
    }

    pub fn push_styled(mut self, content: impl Into<Cow<'static, str>>, style: Style) -> Self {
        self.spans.push(Span::styled(content, style));
        self
    }

    pub fn width(&self) -> usize {
        self.spans.iter().map(Span::width).sum()
    }

    pub fn extend(mut self, other: Line) -> Self {
        self.spans.extend(other.spans);
        self
    }
}

impl<T: Into<Cow<'static, str>>> From<T> for Line {
    fn from(value: T) -> Self {
        Self::raw(value)
    }
}

impl From<Span> for Line {
    fn from(span: Span) -> Self {
        Self { spans: vec![span] }
    }
}

impl Render for Line {
    fn render(&self, ctx: &mut RenderContext<'_>) -> io::Result<()> {
        for span in &self.spans {
            span.render(ctx)?;
        }

        Ok(())
    }
}

impl Display for Line {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for span in &self.spans {
            write!(f, "{span}")?;
        }

        Ok(())
    }
}
