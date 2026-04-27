mod fields;
mod line;
mod span;
mod stack;
mod text;

pub use fields::{Field, Fields};
pub use line::Line;
pub use span::Span;
pub use stack::Stack;
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
    pub fn new(writer: &'a mut dyn Write, color: bool) -> Self {
        Self { writer, color }
    }
}

/// A [`Display`] wrapper for [`Render`] values.
pub struct RenderDisplay<T> {
    renderable: T,
    color: bool,
}

impl<T> RenderDisplay<T> {
    pub fn new(renderable: T) -> Self {
        Self::with_color(renderable, false)
    }

    pub fn with_color(renderable: T, color: bool) -> Self {
        Self { renderable, color }
    }
}

impl<T: Render> Display for RenderDisplay<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut buf = Vec::new();
        let mut ctx = RenderContext::new(&mut buf, self.color);

        self.renderable.render(&mut ctx).map_err(|_| fmt::Error)?;
        let text = String::from_utf8(buf).map_err(|_| fmt::Error)?;

        f.write_str(&text)
    }
}

pub trait RenderExt: Render + Sized {
    fn display(self) -> RenderDisplay<Self> {
        RenderDisplay::new(self)
    }

    fn display_with_color(self, color: bool) -> RenderDisplay<Self> {
        RenderDisplay::with_color(self, color)
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
