use std::fmt::{self, Display, Formatter};
use std::io::{self, Write};

use anyhow::Result;
use clap::ValueEnum;
use serde::ser::SerializeSeq;
use serde::{Serialize, Serializer};
use serde_json::json;

use crate::t;

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

    /// Writes a list of structured data to stdout.
    pub fn print_list(
        &self,
        values: impl IntoIterator<Item = impl Display + Serialize>,
    ) -> Result<()> {
        self.print_list_to(&mut io::stdout(), values)
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

    /// Writes a list of structured data to the given writer.
    pub fn print_list_to(
        &self,
        w: &mut impl Write,
        values: impl IntoIterator<Item = impl Display + Serialize>,
    ) -> Result<()> {
        match self.format {
            Format::Text => {
                for value in values {
                    writeln!(w, "{value}")?;
                }
            }
            Format::Json => {
                let mut serializer = serde_json::Serializer::pretty(&mut *w);
                let mut seq = serializer.serialize_seq(None)?;

                for value in values {
                    seq.serialize_element(&value)?;
                }

                seq.end()?;
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
            Format::Text => writeln!(w, "{}", t!("error-prefix", message = format!("{err:#}"))),
            Format::Json => {
                serde_json::to_writer_pretty(&mut *w, &json!({ "error": format!("{err:#}") }))
                    .map_err(io::Error::other)?;
                writeln!(w)
            }
        }
    }

    pub fn hint(&self, message: &str) {
        if let Err(write_err) = self.hint_to(&mut io::stderr(), message) {
            tracing::warn!(%write_err, "failed to write hint");
        }
    }

    pub fn hint_to(&self, w: &mut impl Write, message: &str) -> io::Result<()> {
        match self.format {
            Format::Text => writeln!(w, "{}", t!("hint-prefix", message = message)),
            Format::Json => Ok(()),
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
    fn print_list_text() {
        let mut buf = Vec::new();
        Output::new(Format::Text)
            .print_list_to(&mut buf, [sample(), sample()])
            .unwrap();

        assert_eq!(
            String::from_utf8(buf).unwrap(),
            indoc! {"
                name:    test
                version: 1
                name:    test
                version: 1
            "}
        );
    }

    #[test]
    fn print_list_json() {
        let mut buf = Vec::new();
        Output::new(Format::Json)
            .print_list_to(&mut buf, [sample(), sample()])
            .unwrap();

        assert_eq!(
            String::from_utf8(buf).unwrap(),
            indoc! {r#"
                [
                  {
                    "name": "test",
                    "version": 1
                  },
                  {
                    "name": "test",
                    "version": 1
                  }
                ]
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

    #[test]
    fn hint_text() {
        let mut buf = Vec::new();
        Output::new(Format::Text)
            .hint_to(&mut buf, "Run `d1v auth login` to authenticate.")
            .unwrap();

        assert_eq!(
            String::from_utf8(buf).unwrap(),
            "Hint: Run `d1v auth login` to authenticate.\n"
        );
    }

    #[test]
    fn hint_json_is_silent() {
        let mut buf = Vec::new();
        Output::new(Format::Json)
            .hint_to(&mut buf, "some hint")
            .unwrap();

        assert!(buf.is_empty());
    }
}
