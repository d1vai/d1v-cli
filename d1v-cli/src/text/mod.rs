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

/// A [`Display`] wrapper for [`Render`] values. Always emits plain text;
/// ANSI escapes produced by [`Render`] implementations are stripped.
pub struct RenderDisplay<T>(T);

impl<T> RenderDisplay<T> {
    pub fn new(renderable: T) -> Self {
        Self(renderable)
    }
}

impl<T: Render> Display for RenderDisplay<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut buf = Vec::new();
        let mut ctx = RenderContext::new(&mut buf);

        self.0.render(&mut ctx).map_err(|_| fmt::Error)?;
        let text = String::from_utf8(buf).map_err(|_| fmt::Error)?;

        f.write_str(&anstream::adapter::strip_str(&text).to_string())
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
    fn display_error() {
        use std::fmt::Write as _;

        let mut text = String::new();

        assert!(write!(&mut text, "{}", BrokenRender.display()).is_err());
    }
}
