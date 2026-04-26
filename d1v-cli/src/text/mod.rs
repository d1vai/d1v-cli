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
