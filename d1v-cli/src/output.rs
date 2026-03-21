use std::fmt::{self, Display, Formatter};
use std::io::{self, Write};

use anyhow::Result;
use clap::ValueEnum;
use serde::Serialize;
use serde_json::json;

/// Output format.
#[derive(Debug, Copy, Clone, Default, ValueEnum)]
pub enum Format {
    /// Plain text
    #[default]
    Text,
    /// JSON
    Json,
}

impl Display for Format {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Format::Text => write!(f, "text"),
            Format::Json => write!(f, "json"),
        }
    }
}

/// Structured output formatter.
#[derive(Debug, Clone)]
pub struct Output {
    pub format: Format,
}

impl Output {
    pub fn new(format: Format) -> Self {
        Self { format }
    }

    /// Writes a status message (stdout in text mode, stderr in JSON mode).
    pub fn message(&self, msg: impl Display) {
        match self.format {
            Format::Text => writeln!(io::stdout(), "{msg}"),
            Format::Json => writeln!(io::stderr(), "{msg}"),
        }
        .unwrap_or_else(|err| tracing::warn!(%err, "failed to write message"));
    }

    /// Writes structured data to stdout ([`Display`] for text, [`Serialize`] for JSON).
    pub fn print(&self, value: &(impl Display + Serialize)) -> Result<()> {
        self.print_to(&mut io::stdout(), value)
    }

    /// Writes structured data to the given writer.
    pub fn print_to(&self, w: &mut impl Write, value: &(impl Display + Serialize)) -> Result<()> {
        match self.format {
            Format::Text => writeln!(w, "{value}")?,
            Format::Json => {
                serde_json::to_writer_pretty(&mut *w, value)?;
                writeln!(w)?;
            }
        }
        Ok(())
    }

    /// Writes an error to stderr in the appropriate format.
    pub fn error(&self, err: &anyhow::Error) {
        if let Err(write_err) = self.error_to(&mut io::stderr(), err) {
            tracing::warn!(%write_err, "failed to write error");
        }
    }

    /// Writes an error to the given writer.
    pub fn error_to(&self, w: &mut impl Write, err: &anyhow::Error) -> io::Result<()> {
        match self.format {
            Format::Text => writeln!(w, "Error: {err:#}"),
            Format::Json => {
                serde_json::to_writer_pretty(&mut *w, &json!({ "error": format!("{err:#}") }))
                    .map_err(io::Error::other)?;
                writeln!(w)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;
    use indoc::indoc;

    #[derive(Debug, Serialize)]
    struct Info {
        name: String,
        version: u32,
    }

    impl Display for Info {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            writeln!(f, "name:    {}", self.name)?;
            write!(f, "version: {}", self.version)
        }
    }

    fn sample() -> Info {
        Info {
            name: "test".into(),
            version: 1,
        }
    }

    #[test]
    fn default_format() {
        assert!(matches!(Format::default(), Format::Text));
    }

    #[test]
    fn print_text() {
        let mut buf = Vec::new();
        Output::new(Format::Text)
            .print_to(&mut buf, &sample())
            .unwrap();

        assert_eq!(
            String::from_utf8(buf).unwrap(),
            indoc! {"
                name:    test
                version: 1
            "}
        );
    }

    #[test]
    fn print_json() {
        let mut buf = Vec::new();
        Output::new(Format::Json)
            .print_to(&mut buf, &sample())
            .unwrap();

        assert_eq!(
            String::from_utf8(buf).unwrap(),
            indoc! {r#"
                {
                  "name": "test",
                  "version": 1
                }
            "#}
        );
    }

    #[test]
    fn error_text() {
        let mut buf = Vec::new();
        Output::new(Format::Text)
            .error_to(&mut buf, &anyhow!("something broke"))
            .unwrap();

        assert_eq!(String::from_utf8(buf).unwrap(), "Error: something broke\n");
    }

    #[test]
    fn error_json() {
        let mut buf = Vec::new();
        Output::new(Format::Json)
            .error_to(&mut buf, &anyhow!("something broke"))
            .unwrap();

        assert_eq!(
            String::from_utf8(buf).unwrap(),
            indoc! {r#"
                {
                  "error": "something broke"
                }
            "#}
        );
    }
}
