use std::borrow::Cow;
use std::io;

use anstyle::Style;

use super::{Line, Render, RenderContext, Span};

pub struct TableRow {
    cells: Vec<Line>,
}

impl TableRow {
    pub fn new<C>(cells: impl IntoIterator<Item = C>) -> Self
    where
        C: Into<Line>,
    {
        Self {
            cells: cells.into_iter().map(Into::into).collect(),
        }
    }
}

pub struct Table {
    header: Option<TableRow>,
    rows: Vec<TableRow>,
    indent: usize,
    border_style: Style,
}

impl Table {
    pub fn new(rows: impl IntoIterator<Item = TableRow>) -> Self {
        Self {
            header: None,
            rows: rows.into_iter().collect(),
            indent: 0,
            border_style: Style::new(),
        }
    }

    pub fn header(mut self, header: TableRow) -> Self {
        self.header = Some(header);
        self
    }

    pub fn indent(mut self, indent: usize) -> Self {
        self.indent = indent;
        self
    }

    pub fn border_style(mut self, style: Style) -> Self {
        self.border_style = style;
        self
    }

    fn column_widths(&self) -> Vec<usize> {
        let columns = self
            .header
            .iter()
            .chain(self.rows.iter())
            .map(|row| row.cells.len())
            .max()
            .unwrap_or(0);

        let mut widths = vec![0; columns];
        for row in self.header.iter().chain(self.rows.iter()) {
            for (index, cell) in row.cells.iter().enumerate() {
                widths[index] = widths[index].max(cell.width());
            }
        }

        widths
    }

    fn render_border(
        &self,
        ctx: &mut RenderContext<'_>,
        text: impl Into<Cow<'static, str>>,
    ) -> io::Result<()> {
        Span::styled(text, self.border_style).render(ctx)
    }

    fn render_indent(&self, ctx: &mut RenderContext<'_>) -> io::Result<()> {
        write!(ctx.writer, "{:indent$}", "", indent = self.indent)
    }

    fn render_rule(
        &self,
        ctx: &mut RenderContext<'_>,
        left: &'static str,
        separator: &'static str,
        right: &'static str,
        widths: &[usize],
    ) -> io::Result<()> {
        self.render_indent(ctx)?;
        self.render_border(ctx, left)?;
        for (index, width) in widths.iter().enumerate() {
            if index > 0 {
                self.render_border(ctx, separator)?;
            }
            self.render_border(ctx, "─".repeat(width + 2))?;
        }
        self.render_border(ctx, right)?;
        writeln!(ctx.writer)
    }

    fn render_row(
        &self,
        ctx: &mut RenderContext<'_>,
        row: &TableRow,
        widths: &[usize],
    ) -> io::Result<()> {
        self.render_indent(ctx)?;
        self.render_border(ctx, "│")?;

        for (index, width) in widths.iter().enumerate() {
            if index > 0 {
                self.render_border(ctx, "│")?;
            }

            write!(ctx.writer, " ")?;
            if let Some(cell) = row.cells.get(index) {
                cell.render(ctx)?;
                write!(
                    ctx.writer,
                    "{:pad$}",
                    "",
                    pad = width.saturating_sub(cell.width())
                )?;
            } else {
                write!(ctx.writer, "{:pad$}", "", pad = width)?;
            }
            write!(ctx.writer, " ")?;
        }

        self.render_border(ctx, "│")?;
        writeln!(ctx.writer)
    }
}

impl Render for Table {
    fn render(&self, ctx: &mut RenderContext<'_>) -> io::Result<()> {
        if self.header.is_none() && self.rows.is_empty() {
            return Ok(());
        }

        let widths = self.column_widths();

        self.render_rule(ctx, "┌", "┬", "┐", &widths)?;

        if let Some(header) = &self.header {
            self.render_row(ctx, header, &widths)?;
            self.render_rule(ctx, "├", "┼", "┤", &widths)?;
        }

        for row in &self.rows {
            self.render_row(ctx, row, &widths)?;
        }

        self.render_rule(ctx, "└", "┴", "┘", &widths)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::RenderExt;

    #[test]
    fn two_column_grid() {
        let table = Table::new([
            TableRow::new(["Status", "Authenticated"]),
            TableRow::new(["Source", "keyring"]),
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

    #[test]
    fn empty_table() {
        let table = Table::new(std::iter::empty());

        assert_eq!(table.display().to_string(), "");
    }

    #[test]
    fn with_header() {
        let table = Table::new([
            TableRow::new(["1", "d1v"]),
            TableRow::new(["2", "mock-user"]),
        ])
        .header(TableRow::new(["id", "slug"]));

        assert_eq!(
            table.display().to_string(),
            concat!(
                "┌────┬───────────┐\n",
                "│ id │ slug      │\n",
                "├────┼───────────┤\n",
                "│ 1  │ d1v       │\n",
                "│ 2  │ mock-user │\n",
                "└────┴───────────┘\n",
            )
        );
    }
}
