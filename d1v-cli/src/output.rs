use std::fmt::{self, Display, Formatter};
use std::io::{self, Write};

use anstream::stream::{AsLockedWrite, RawStream};
use anstream::{AutoStream, ColorChoice};
use clap::ValueEnum;
use serde::Serialize;
use serde_json::json;

use crate::error::Result;
use crate::symbols;
use crate::t;
use crate::text::{Line, Render, RenderContext};
use crate::theme;

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
    pub color: ColorChoice,
}

struct StatusLine<M> {
    symbol: &'static str,
    message: M,
    style: theme::ansi::Style,
}

impl<M: Display> Render for StatusLine<M> {
    fn render(&self, ctx: &mut RenderContext<'_>) -> io::Result<()> {
        Line::new()
            .push_styled(self.symbol, self.style)
            .push_plain(" ")
            .push_styled(self.message.to_string(), self.style)
            .render(ctx)?;

        writeln!(ctx.writer)
    }
}

impl Output {
    pub fn new(format: Format, color: ColorChoice) -> Self {
        Self { format, color }
    }

    fn auto<W: RawStream>(&self, w: W) -> AutoStream<W> {
        AutoStream::new(w, self.color)
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
                let mut writer = RenderContext::new(&mut out);
                text.render(&mut writer)?;
            }
            Format::Json => Self::write_json(&mut out, json)?,
        }
        Ok(())
    }

    pub fn success(&self, msg: impl Display) {
        match self.format {
            Format::Text => {
                let mut out = self.auto(io::stdout());
                let mut ctx = RenderContext::new(&mut out);
                StatusLine {
                    symbol: symbols::SUCCESS,
                    message: msg,
                    style: theme::ansi::success(),
                }
                .render(&mut ctx)
            }
            Format::Json => writeln!(io::stderr(), "{msg}"),
        }
        .unwrap_or_else(|err| tracing::warn!(%err, "failed to write success message"));
    }

    /// Writes an informational message (e.g., "code sent", "email delivered").
    pub fn info(&self, msg: impl Display) {
        match self.format {
            Format::Text => {
                let mut out = self.auto(io::stdout());
                let mut ctx = RenderContext::new(&mut out);
                StatusLine {
                    symbol: symbols::INFO,
                    message: msg,
                    style: theme::ansi::info(),
                }
                .render(&mut ctx)
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
                let mut ctx = RenderContext::new(&mut out);
                StatusLine {
                    symbol: symbols::INFO,
                    message: msg,
                    style: theme::ansi::info(),
                }
                .render(&mut ctx)
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
                let mut ctx = RenderContext::new(&mut out);
                StatusLine {
                    symbol: symbols::ERROR,
                    message: err,
                    style: theme::ansi::error(),
                }
                .render(&mut ctx)
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
                let mut ctx = RenderContext::new(&mut out);
                Line::new()
                    .push_plain("  ")
                    .push_styled(message.to_owned(), theme::ansi::hint())
                    .render(&mut ctx)?;
                writeln!(out)
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
        Output::new(Format::Text, ColorChoice::Never)
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
        Output::new(Format::Json, ColorChoice::Never)
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
        let err = Error::Api(APIError::api(1, "something broke"));
        Output::new(Format::Text, ColorChoice::Never)
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
        let err = Error::Api(APIError::api(1, "something broke"));
        Output::new(Format::Json, ColorChoice::Never)
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
        Output::new(Format::Text, ColorChoice::Never)
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
        Output::new(Format::Json, ColorChoice::Never)
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
        Output::new(Format::Text, ColorChoice::Never)
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
        Output::new(Format::Json, ColorChoice::Never)
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
