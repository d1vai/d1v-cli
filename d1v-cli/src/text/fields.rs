use std::io;

use super::{Render, RenderContext, Span};

pub struct Field {
    pub label: Span,
    pub value: Span,
}

impl Field {
    pub fn new(label: impl Into<Span>, value: impl Into<Span>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }
}

pub struct Fields {
    fields: Vec<Field>,
    indent: usize,
    gap: usize,
}

impl Fields {
    pub fn new(fields: impl IntoIterator<Item = Field>) -> Self {
        Self {
            fields: fields.into_iter().collect(),
            indent: 0,
            gap: 2,
        }
    }

    pub fn indent(mut self, indent: usize) -> Self {
        self.indent = indent;
        self
    }

    pub fn gap(mut self, gap: usize) -> Self {
        self.gap = gap;
        self
    }
}

impl Render for Fields {
    fn render(&self, ctx: &mut RenderContext<'_>) -> io::Result<()> {
        let width = self
            .fields
            .iter()
            .map(|field| field.label.width())
            .max()
            .unwrap_or(0);

        for field in &self.fields {
            let pad = width.saturating_sub(field.label.width()) + self.gap;
            write!(ctx.writer, "{:indent$}", "", indent = self.indent)?;
            field.label.render(ctx)?;
            write!(ctx.writer, "{:pad$}", "", pad = pad)?;
            field.value.render(ctx)?;
            writeln!(ctx.writer)?;
        }

        Ok(())
    }
}
