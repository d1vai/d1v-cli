use std::fmt::{self, Display, Formatter};
use std::io;

use super::{Line, Render, RenderContext};

/// Vertical sequence of [`Line`]s. Renders each line followed by a newline.
#[derive(Default)]
pub struct Text {
    pub lines: Vec<Line>,
}

impl Text {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn line(mut self, line: impl Into<Line>) -> Self {
        self.lines.push(line.into());
        self
    }

    pub fn lines<L>(mut self, lines: impl IntoIterator<Item = L>) -> Self
    where
        L: Into<Line>,
    {
        self.lines.extend(lines.into_iter().map(Into::into));
        self
    }
}

impl<T: Into<Line>> From<T> for Text {
    fn from(value: T) -> Self {
        Self {
            lines: vec![value.into()],
        }
    }
}

impl<L: Into<Line>> FromIterator<L> for Text {
    fn from_iter<I: IntoIterator<Item = L>>(iter: I) -> Self {
        Self {
            lines: iter.into_iter().map(Into::into).collect(),
        }
    }
}

impl Render for Text {
    fn render(&self, ctx: &mut RenderContext<'_>) -> io::Result<()> {
        for line in &self.lines {
            line.render(ctx)?;
            writeln!(ctx.writer)?;
        }

        Ok(())
    }
}

impl Display for Text {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for line in &self.lines {
            writeln!(f, "{line}")?;
        }

        Ok(())
    }
}
