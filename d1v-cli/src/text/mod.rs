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
}

impl<'a> RenderContext<'a> {
    pub fn new(writer: &'a mut dyn Write) -> Self {
        Self { writer }
    }
}

/// A [`Display`] adapter that renders a [`Render`] value as plain text.
///
/// ANSI styling emitted during rendering is stripped.
pub struct RenderDisplay<T>(T);

impl<T> RenderDisplay<T> {
    pub fn new(renderable: T) -> Self {
        Self(renderable)
    }
}

impl<T: Render> Display for RenderDisplay<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut writer = anstream::StripStream::new(Vec::new());
        let mut ctx = RenderContext::new(&mut writer);

        self.0.render(&mut ctx).map_err(|_| fmt::Error)?;
        let buf = writer.into_inner();
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
            ctx.writer.write_all(b"\x1b[32mhello\x1b[0m")
        }
    }

    struct SplitRender;

    impl Render for SplitRender {
        fn render(&self, ctx: &mut RenderContext<'_>) -> io::Result<()> {
            for byte in "\x1b[32m✓\x1b[0m".as_bytes() {
                ctx.writer.write_all(&[*byte])?;
            }

            Ok(())
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
    fn display_strips_ansi() {
        assert_eq!(AnsiRender.display().to_string(), "hello");
    }

    #[test]
    fn display_handles_split_writes() {
        assert_eq!(SplitRender.display().to_string(), "✓");
    }

    #[test]
    fn display_error() {
        use std::fmt::Write as _;

        let mut text = String::new();

        assert!(write!(&mut text, "{}", BrokenRender.display()).is_err());
    }
}
