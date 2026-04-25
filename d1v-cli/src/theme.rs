pub mod ansi {
    use std::fmt::{self, Display, Formatter};
    use std::io::{self, IsTerminal};
    use std::sync::atomic::{AtomicU8, Ordering};

    use anstyle::{Color, RgbColor};
    use colorgrad::Gradient;

    pub use anstyle::Style;

    const fn rgb(r: u8, g: u8, b: u8) -> Style {
        Style::new().fg_color(Some(Color::Rgb(RgbColor(r, g, b))))
    }

    pub const fn success() -> Style {
        rgb(35, 158, 98)
    }

    pub const fn error() -> Style {
        rgb(221, 57, 98)
    }

    pub const fn warning() -> Style {
        rgb(202, 186, 45)
    }

    pub const fn info() -> Style {
        rgb(89, 139, 255)
    }

    pub const fn hint() -> Style {
        rgb(165, 163, 180)
    }

    pub const fn label() -> Style {
        rgb(208, 208, 217).bold()
    }

    pub const fn value() -> Style {
        rgb(178, 121, 242)
    }

    pub const fn dim() -> Style {
        rgb(109, 106, 128)
    }

    #[derive(Debug, Copy, Clone)]
    pub enum Stream {
        Stdout,
        Stderr,
    }

    const OVERRIDE_SET: u8 = 0b10;
    const OVERRIDE_ON: u8 = 0b01;
    static OVERRIDE: AtomicU8 = AtomicU8::new(0);

    pub fn set_override(enabled: bool) {
        let bits = OVERRIDE_SET | if enabled { OVERRIDE_ON } else { 0 };
        OVERRIDE.store(bits, Ordering::Relaxed);
    }

    pub fn unset_override() {
        OVERRIDE.store(0, Ordering::Relaxed);
    }

    fn override_state() -> (bool, bool) {
        let bits = OVERRIDE.load(Ordering::Relaxed);
        let set = bits & OVERRIDE_SET != 0;
        let on = bits & OVERRIDE_ON != 0;
        (set && on, set && !on)
    }

    pub fn supports_color(stream: Stream) -> bool {
        let (force_on, force_off) = override_state();

        if force_on {
            return true;
        }

        if force_off || std::env::var_os("NO_COLOR").is_some() {
            return false;
        }

        match stream {
            Stream::Stdout => io::stdout().is_terminal(),
            Stream::Stderr => io::stderr().is_terminal(),
        }
    }

    pub struct Styled<T: ?Sized> {
        style: Style,
        value: T,
    }

    impl<T: Display + ?Sized> Display for Styled<T> {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            write!(
                f,
                "{}{}{}",
                self.style.render(),
                &self.value,
                self.style.render_reset()
            )
        }
    }

    pub struct ColorDisplay<'a, T, Out, F>(&'a T, Stream, F)
    where
        T: ?Sized,
        F: Fn(&'a T) -> Out;

    impl<'a, T, Out, F> Display for ColorDisplay<'a, T, Out, F>
    where
        T: Display + ?Sized,
        Out: Display,
        F: Fn(&'a T) -> Out,
    {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            if supports_color(self.1) {
                (self.2)(self.0).fmt(f)
            } else {
                self.0.fmt(f)
            }
        }
    }

    pub trait Stylize {
        fn style(&self, style: Style) -> Styled<&Self>;

        fn truecolor(&self, r: u8, g: u8, b: u8) -> Styled<&Self> {
            self.style(rgb(r, g, b))
        }

        fn if_supports_color<'a, Out, F>(
            &'a self,
            stream: Stream,
            apply: F,
        ) -> ColorDisplay<'a, Self, Out, F>
        where
            F: Fn(&'a Self) -> Out;
    }

    impl<T: ?Sized> Stylize for T {
        fn style(&self, style: Style) -> Styled<&Self> {
            Styled { style, value: self }
        }

        fn if_supports_color<'a, Out, F>(
            &'a self,
            stream: Stream,
            apply: F,
        ) -> ColorDisplay<'a, Self, Out, F>
        where
            F: Fn(&'a Self) -> Out,
        {
            ColorDisplay(self, stream, apply)
        }
    }

    /// Applies a color gradient across each character of a string.
    pub fn gradient(text: impl AsRef<str>, gradient: &impl Gradient) -> String {
        let text = text.as_ref();

        gradient
            .colors(text.chars().count())
            .into_iter()
            .zip(text.chars())
            .map(|(color, ch)| {
                let [r, g, b, _] = color.to_rgba8();
                ch.if_supports_color(Stream::Stdout, |s| s.style(rgb(r, g, b)))
                    .to_string()
            })
            .collect()
    }
}

pub mod tui {
    use ratatui::style::{Color, Style};

    // DarkPurple palette
    const ACCENT: Color = Color::Rgb(178, 121, 242); // #B279F2
    const FOREGROUND: Color = Color::Rgb(208, 208, 217); // #D0D0D9
    const DIM: Color = Color::Rgb(109, 106, 128); // #6D6A80
    const SUCCESS_GREEN: Color = Color::Rgb(35, 158, 98); // #239E62
    const ERROR_RED: Color = Color::Rgb(221, 57, 98); // #DD3962
    const DESCRIPTION: Color = Color::Rgb(165, 163, 180); // #A5A3B4
    const KEY: Color = Color::Rgb(160, 155, 180); // #A09BB4

    pub const fn success() -> Style {
        Style::new().fg(SUCCESS_GREEN).bold()
    }

    pub const fn error() -> Style {
        Style::new().fg(ERROR_RED)
    }

    pub const fn prompt() -> Style {
        Style::new().fg(ACCENT)
    }

    pub const fn label() -> Style {
        Style::new().fg(FOREGROUND).bold()
    }

    pub const fn key() -> Style {
        Style::new().fg(KEY)
    }

    pub const fn value() -> Style {
        Style::new().fg(ACCENT)
    }

    pub const fn dim() -> Style {
        Style::new().fg(DIM)
    }

    pub const fn active() -> Style {
        Style::new().fg(ACCENT).bold().underlined()
    }

    pub const fn inactive() -> Style {
        Style::new()
    }

    pub const fn description() -> Style {
        Style::new().fg(DESCRIPTION)
    }
}
