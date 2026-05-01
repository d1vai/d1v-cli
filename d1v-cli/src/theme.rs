pub mod ansi {
    use anstyle::{Color, RgbColor};
    use colorgrad::Gradient;

    use crate::text::Line;

    pub use anstyle::Style;

    pub const fn rgb(r: u8, g: u8, b: u8) -> Style {
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

    pub const fn plain() -> Style {
        rgb(232, 232, 238)
    }

    pub const fn border() -> Style {
        rgb(62, 59, 74)
    }

    /// Applies a color gradient across each character of a string.
    pub fn gradient_line(text: impl AsRef<str>, gradient: &impl Gradient) -> Line {
        let text = text.as_ref();
        let mut line = Line::default();

        for (color, character) in gradient
            .colors(text.chars().count())
            .into_iter()
            .zip(text.chars())
        {
            let [r, g, b, _] = color.to_rgba8();
            line = line.push_styled(character.to_string(), rgb(r, g, b));
        }

        line
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
