use std::io;

use anstyle::Style;

use super::{Line, Render, RenderContext, Span};

pub struct TableRow {
    pub label: Span,
    pub value: Line,
}

impl TableRow {
    pub fn new(label: impl Into<Span>, value: impl Into<Line>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }
}

pub struct Table {
    rows: Vec<TableRow>,
    indent: usize,
    border_style: Style,
}

impl Table {
    pub fn new(rows: impl IntoIterator<Item = TableRow>) -> Self {
        Self {
            rows: rows.into_iter().collect(),
            indent: 0,
            border_style: Style::new(),
        }
    }

    pub fn indent(mut self, indent: usize) -> Self {
        self.indent = indent;
        self
    }

    pub fn border_style(mut self, style: Style) -> Self {
        self.border_style = style;
        self
    }

    fn label_width(&self) -> usize {
        self.rows
            .iter()
            .map(|row| row.label.width())
            .max()
            .unwrap_or(0)
    }

    fn value_width(&self) -> usize {
        self.rows
            .iter()
            .map(|row| row.value.width())
            .max()
            .unwrap_or(0)
    }

    fn render_border(
        &self,
        ctx: &mut RenderContext<'_>,
        text: impl Into<String>,
    ) -> io::Result<()> {
        Span::styled(text.into(), self.border_style).render(ctx)
    }

    fn render_indent(&self, ctx: &mut RenderContext<'_>) -> io::Result<()> {
        write!(ctx.writer, "{:indent$}", "", indent = self.indent)
    }
}

impl Render for Table {
    fn render(&self, ctx: &mut RenderContext<'_>) -> io::Result<()> {
        let label_width = self.label_width();
        let value_width = self.value_width();

        self.render_indent(ctx)?;
        self.render_border(ctx, "┌")?;
        self.render_border(ctx, "─".repeat(label_width + 2))?;
        self.render_border(ctx, "┬")?;
        self.render_border(ctx, "─".repeat(value_width + 2))?;
        self.render_border(ctx, "┐")?;
        writeln!(ctx.writer)?;

        for row in &self.rows {
            self.render_indent(ctx)?;
            self.render_border(ctx, "│ ")?;

            row.label.render(ctx)?;
            write!(
                ctx.writer,
                "{:pad$}",
                "",
                pad = label_width.saturating_sub(row.label.width())
            )?;

            self.render_border(ctx, " │ ")?;

            row.value.render(ctx)?;
            write!(
                ctx.writer,
                "{:pad$}",
                "",
                pad = value_width.saturating_sub(row.value.width())
            )?;

            self.render_border(ctx, " │")?;
            writeln!(ctx.writer)?;
        }

        self.render_indent(ctx)?;
        self.render_border(ctx, "└")?;
        self.render_border(ctx, "─".repeat(label_width + 2))?;
        self.render_border(ctx, "┴")?;
        self.render_border(ctx, "─".repeat(value_width + 2))?;
        self.render_border(ctx, "┘")?;
        writeln!(ctx.writer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::RenderExt;

    #[test]
    fn renders_two_column_grid() {
        let table = Table::new([
            TableRow::new("Status", "Authenticated"),
            TableRow::new("Source", "keyring"),
        ]);

        assert_eq!(
            table.display().to_string(),
            concat!(
                "┌────────┬───────────────┐\n",
                "│ Status │ Authenticated │\n",
                "│ Source │ keyring       │\n",
                "└────────┴───────────────┘\n",
            )
        );
    }
}
