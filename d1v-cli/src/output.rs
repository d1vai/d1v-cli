use std::fmt::{self, Display, Formatter};
use std::io::{self, IsTerminal, Write};

use anstream::stream::{AsLockedWrite, RawStream};
use anstream::AutoStream;
use anstyle::Style;
use clap::ValueEnum;
use serde::Serialize;
use serde_json::json;

use crate::error::Result;
use crate::symbols;
use crate::t;
use crate::text::{Render, RenderContext};
use crate::theme;
use crate::theme::ansi::Stylize;

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

/// Color output preference.
#[derive(Debug, Copy, Clone, Default, ValueEnum)]
pub enum Color {
    /// Enable colors when writing to a terminal
    #[default]
    Auto,
    /// Always emit ANSI color codes
    Always,
    /// Never emit ANSI color codes
    Never,
}

impl Color {
    /// Resolves to a concrete boolean based on terminal capabilities.
    pub fn resolve(self) -> bool {
        match self {
            Self::Always => true,
            Self::Never => false,
            Self::Auto => std::env::var_os("NO_COLOR").is_none() && io::stderr().is_terminal(),
        }
    }
}

impl Display for Color {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Color::Auto => write!(f, "auto"),
            Color::Always => write!(f, "always"),
            Color::Never => write!(f, "never"),
        }
    }
}

/// Formats seconds into a localized human-readable duration (e.g., `2d 3h`).
pub fn format_duration(total_secs: i64) -> String {
    let days = total_secs / 86400;
    let hours = (total_secs % 86400) / 3600;
    let minutes = (total_secs % 3600) / 60;

    if days > 0 {
        t!("duration-days-hours", days = days, hours = hours)
    } else if hours > 0 {
        t!("duration-hours-minutes", hours = hours, minutes = minutes)
    } else {
        t!("duration-minutes", minutes = minutes.max(1))
    }
}

/// Structured output formatter.
#[derive(Debug, Clone)]
pub struct Output {
    pub format: Format,
    pub color: bool,
}

impl Output {
    pub fn new(format: Format, color: bool) -> Self {
        Self { format, color }
    }

    fn choice(&self) -> anstream::ColorChoice {
        if self.color {
            anstream::ColorChoice::Always
        } else {
            anstream::ColorChoice::Never
        }
    }

    fn auto<W: RawStream>(&self, w: W) -> AutoStream<W> {
        AutoStream::new(w, self.choice())
    }

    pub fn present<T, J>(&self, text: T, json: &J) -> Result
    where
        T: Render,
        J: Serialize + ?Sized,
    {
        self.present_to(io::stdout(), text, json)
    }

    pub fn present_to<T, J>(&self, w: impl RawStream + AsLockedWrite, text: T, json: &J) -> Result
    where
        T: Render,
        J: Serialize + ?Sized,
    {
        let mut out = self.auto(w);
        match self.format {
            Format::Text => {
                let mut writer = RenderContext::new(&mut out, self.color);
                text.render(&mut writer)?;
            }
            Format::Json => Self::write_json(&mut out, json)?,
        }
        Ok(())
    }

    pub fn success(&self, msg: impl Display) {
        match self.format {
            Format::Text => {
                let message = format!("{} {msg}", symbols::SUCCESS);
                writeln!(
                    self.auto(io::stdout()),
                    "{}",
                    message.style(theme::ansi::success())
                )
            }
            Format::Json => writeln!(io::stderr(), "{msg}"),
        }
        .unwrap_or_else(|err| tracing::warn!(%err, "failed to write success message"));
    }

    /// Writes an informational message (e.g., "code sent", "email delivered").
    pub fn info(&self, msg: impl Display) {
        match self.format {
            Format::Text => {
                let message = format!("{} {msg}", symbols::INFO);
                writeln!(
                    self.auto(io::stdout()),
                    "{}",
                    message.style(theme::ansi::info())
                )
            }
            Format::Json => writeln!(io::stderr(), "{msg}"),
        }
        .unwrap_or_else(|err| tracing::warn!(%err, "failed to write info message"));
    }

    /// Writes an informational message to the given writer.
    pub fn info_to(&self, w: impl RawStream + AsLockedWrite, msg: impl Display) -> io::Result<()> {
        let mut out = self.auto(w);
        match self.format {
            Format::Text => {
                let message = format!("{} {msg}", symbols::INFO);
                writeln!(out, "{}", message.style(theme::ansi::info()))
            }
            Format::Json => writeln!(out, "{msg}"),
        }
    }

    /// Writes a status message (stdout in text mode, stderr in JSON mode).
    pub fn message(&self, msg: impl Display) {
        match self.format {
            Format::Text => writeln!(io::stdout(), "{msg}"),
            Format::Json => writeln!(io::stderr(), "{msg}"),
        }
        .unwrap_or_else(|err| tracing::warn!(%err, "failed to write message"));
    }

    /// Writes an error to stderr in the appropriate format.
    pub fn error(&self, err: &dyn Display) {
        if let Err(write_err) = self.error_to(io::stderr(), err) {
            tracing::warn!(%write_err, "failed to write error");
        }
    }

    /// Writes an error to the given writer.
    pub fn error_to(&self, w: impl RawStream + AsLockedWrite, err: &dyn Display) -> io::Result<()> {
        let mut out = self.auto(w);
        match self.format {
            Format::Text => {
                let message = format!("{} {err}", symbols::ERROR);
                writeln!(out, "{}", message.style(theme::ansi::error()))
            }
            Format::Json => Self::write_json(&mut out, &json!({ "error": format!("{err}") })),
        }
    }

    pub fn hint(&self, message: &str) {
        if let Err(write_err) = self.hint_to(io::stderr(), message) {
            tracing::warn!(%write_err, "failed to write hint");
        }
    }

    pub fn hint_to(&self, w: impl RawStream + AsLockedWrite, message: &str) -> io::Result<()> {
        let mut out = self.auto(w);
        match self.format {
            Format::Text => {
                let message = message.style(theme::ansi::hint());
                writeln!(out, "  {message}")
            }
            Format::Json => Ok(()),
        }
    }

    fn write_json(w: &mut impl Write, value: &(impl Serialize + ?Sized)) -> io::Result<()> {
        serde_json::to_writer_pretty(&mut *w, value).map_err(io::Error::other)?;
        writeln!(w)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{APIError, Error};
    use crate::text::Text;
    use indoc::indoc;

    #[derive(Debug, Serialize)]
    struct Info {
        name: String,
        version: u32,
    }

    struct InfoView<'a> {
        info: &'a Info,
    }

    impl Render for InfoView<'_> {
        fn render(&self, writer: &mut RenderContext<'_>) -> io::Result<()> {
            Text::new()
                .line(format!("name:    {}", self.info.name))
                .line(format!("version: {}", self.info.version))
                .render(writer)
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
    fn present_text() {
        let info = sample();
        let mut buf = Vec::new();
        Output::new(Format::Text, false)
            .present_to(&mut buf, InfoView { info: &info }, &info)
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
    fn present_json() {
        let info = sample();
        let mut buf = Vec::new();
        Output::new(Format::Json, false)
            .present_to(&mut buf, InfoView { info: &info }, &info)
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
        let err = Error::Api(APIError::Api {
            code: 1.into(),
            message: "something broke".into(),
        });
        Output::new(Format::Text, false)
            .error_to(&mut buf, &err)
            .unwrap();

        assert_eq!(
            String::from_utf8(buf).unwrap(),
            "✗ api error 1: something broke\n"
        );
    }

    #[test]
    fn error_json() {
        let mut buf = Vec::new();
        let err = Error::Api(APIError::Api {
            code: 1.into(),
            message: "something broke".into(),
        });
        Output::new(Format::Json, false)
            .error_to(&mut buf, &err)
            .unwrap();

        assert_eq!(
            String::from_utf8(buf).unwrap(),
            indoc! {r#"
                {
                  "error": "api error 1: something broke"
                }
            "#}
        );
    }

    #[test]
    fn info_text() {
        let mut buf = Vec::new();
        Output::new(Format::Text, false)
            .info_to(&mut buf, "Code sent to user@example.com")
            .unwrap();

        assert_eq!(
            String::from_utf8(buf).unwrap(),
            "→ Code sent to user@example.com\n"
        );
    }

    #[test]
    fn info_json() {
        let mut buf = Vec::new();
        Output::new(Format::Json, false)
            .info_to(&mut buf, "Code sent to user@example.com")
            .unwrap();

        assert_eq!(
            String::from_utf8(buf).unwrap(),
            "Code sent to user@example.com\n"
        );
    }

    #[test]
    fn hint_text() {
        let mut buf = Vec::new();
        Output::new(Format::Text, false)
            .hint_to(&mut buf, "Run `d1v auth login` to authenticate.")
            .unwrap();

        assert_eq!(
            String::from_utf8(buf).unwrap(),
            "  Run `d1v auth login` to authenticate.\n"
        );
    }

    #[test]
    fn hint_json_is_silent() {
        let mut buf = Vec::new();
        Output::new(Format::Json, false)
            .hint_to(&mut buf, "some hint")
            .unwrap();

        assert!(buf.is_empty());
    }

    #[test]
    fn format_duration_days_hours() {
        assert_eq!(format_duration(90000), "1d 1h");
    }

    #[test]
    fn format_duration_hours_minutes() {
        assert_eq!(format_duration(9000), "2h 30m");
    }

    #[test]
    fn format_duration_minutes_only() {
        assert_eq!(format_duration(300), "5m");
    }

    #[test]
    fn format_duration_less_than_minute() {
        assert_eq!(format_duration(30), "1m");
    }
}
