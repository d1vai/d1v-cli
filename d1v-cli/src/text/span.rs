use std::borrow::Cow;
use std::fmt::{self, Display, Formatter};
use std::io;

use anstyle::Style;
use unicode_width::UnicodeWidthStr;

use super::{Render, RenderContext};

/// Atomic styled text fragment.
pub struct Span {
    pub content: Cow<'static, str>,
    pub style: Option<Style>,
}

impl Span {
    pub fn raw(content: impl Into<Cow<'static, str>>) -> Self {
        Self {
            content: content.into(),
            style: None,
        }
    }

    pub fn styled(content: impl Into<Cow<'static, str>>, style: Style) -> Self {
        Self {
            content: content.into(),
            style: Some(style),
        }
    }

    pub fn width(&self) -> usize {
        self.content.width()
    }
}

impl<T: Into<Cow<'static, str>>> From<T> for Span {
    fn from(value: T) -> Self {
        Self::raw(value)
    }
}

impl Render for Span {
    fn render(&self, ctx: &mut RenderContext<'_>) -> io::Result<()> {
        if ctx.color
            && let Some(style) = self.style
        {
            write!(
                ctx.writer,
                "{}{}{}",
                style.render(),
                self.content,
                style.render_reset()
            )
        } else {
            write!(ctx.writer, "{}", self.content)
        }
    }
}

impl Display for Span {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.content)
    }
}
