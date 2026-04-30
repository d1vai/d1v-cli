mod fields;
mod line;
mod span;
mod stack;
mod table;
mod text;

pub use fields::{Field, Fields};
pub use line::Line;
pub use span::Span;
pub use stack::Stack;
pub use table::{Table, TableRow};
pub use text::Text;

use std::fmt::{self, Display, Formatter};
use std::io;
use std::io::Write;

pub trait Render {
    fn render(&self, ctx: &mut RenderContext<'_>) -> io::Result<()>;
}

pub struct RenderContext<'a> {
    pub writer: &'a mut dyn Write,
    pub color: bool,
}

impl<'a> RenderContext<'a> {
    pub fn new(writer: &'a mut dyn Write) -> Self {
        Self {
            writer,
            color: true,
        }
    }

    pub fn color(mut self, color: bool) -> Self {
        self.color = color;
        self
    }
}

/// A [`Display`] adapter that renders a [`Render`] value as plain text.
pub struct RenderDisplay<T>(T);

impl<T> RenderDisplay<T> {
    pub fn new(renderable: T) -> Self {
        Self(renderable)
    }
}

impl<T: Render> Display for RenderDisplay<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut buf = Vec::new();
        let mut ctx = RenderContext::new(&mut buf).color(false);

        self.0.render(&mut ctx).map_err(|_| fmt::Error)?;
        let text = std::str::from_utf8(&buf).map_err(|_| fmt::Error)?;

        f.write_str(text)
    }
}

pub trait RenderExt: Render + Sized {
    fn display(self) -> RenderDisplay<Self> {
        RenderDisplay::new(self)
    }
}

impl<T: Render> RenderExt for T {}

#[cfg(test)]
mod tests {
    use super::*;

    struct BrokenRender;

    impl Render for BrokenRender {
        fn render(&self, _ctx: &mut RenderContext<'_>) -> io::Result<()> {
            Err(io::Error::other("render failed"))
        }
    }

    struct AnsiRender;

    impl Render for AnsiRender {
        fn render(&self, ctx: &mut RenderContext<'_>) -> io::Result<()> {
            if ctx.color {
                ctx.writer.write_all(b"\x1b[32mhello\x1b[0m")
            } else {
                ctx.writer.write_all(b"hello")
            }
        }
    }

    #[test]
    fn display_plain_text() {
        let rendered = Text::new()
            .line("hello")
            .line("world")
            .display()
            .to_string();

        assert_eq!(rendered, "hello\nworld\n");
    }

    #[test]
    fn display_plain_source() {
        assert_eq!(AnsiRender.display().to_string(), "hello");
    }

    #[test]
    fn span_display_plain() {
        let span = Span::styled("hello", crate::theme::ansi::success());

        assert_eq!(span.to_string(), "hello");
    }

    #[test]
    fn line_display_plain() {
        let line = Line::new()
            .push_styled("hello", crate::theme::ansi::success())
            .push_plain(" world");

        assert_eq!(line.to_string(), "hello world");
    }

    #[test]
    fn text_display_plain() {
        let text = Text::new()
            .line(Line::styled("hello", crate::theme::ansi::success()))
            .line(Line::styled("world", crate::theme::ansi::info()));

        assert_eq!(text.to_string(), "hello\nworld\n");
    }

    #[test]
    fn display_error() {
        use std::fmt::Write as _;

        let mut text = String::new();

        assert!(write!(&mut text, "{}", BrokenRender.display()).is_err());
    }
}
