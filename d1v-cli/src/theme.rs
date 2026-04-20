pub mod owo {
    use colorgrad::Gradient;
    use owo_colors::{OwoColorize, Stream, Style};

    pub const fn success() -> Style {
        Style::new().green()
    }

    pub const fn error() -> Style {
        Style::new().bright_red()
    }

    pub const fn info() -> Style {
        Style::new().cyan()
    }

    pub const fn hint() -> Style {
        Style::new().yellow()
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
                ch.if_supports_color(Stream::Stdout, |s| s.truecolor(r, g, b))
                    .to_string()
            })
            .collect()
    }
}

pub mod tui {
    use ratatui::style::{Color, Style};

    pub const fn success() -> Style {
        Style::new().fg(Color::Green).bold()
    }

    pub const fn error() -> Style {
        Style::new().fg(Color::LightRed)
    }

    pub const fn prompt() -> Style {
        Style::new().fg(Color::Green)
    }

    pub const fn label() -> Style {
        Style::new().fg(Color::Green).bold()
    }

    pub const fn value() -> Style {
        Style::new().fg(Color::Cyan)
    }

    pub const fn dim() -> Style {
        Style::new().fg(Color::DarkGray)
    }

    pub const fn toggle_active() -> Style {
        Style::new().fg(Color::Cyan).bold().underlined()
    }

    pub const fn toggle_inactive() -> Style {
        Style::new().fg(Color::DarkGray)
    }

    pub const fn select_label() -> Style {
        Style::new().fg(Color::Rgb(208, 208, 217)).bold()
    }

    pub const fn select_arrow() -> Style {
        Style::new().fg(Color::Rgb(178, 121, 242))
    }

    pub const fn select_active() -> Style {
        Style::new().fg(Color::Rgb(178, 121, 242))
    }

    pub const fn select_inactive() -> Style {
        Style::new()
    }

    pub const fn select_dim() -> Style {
        Style::new().fg(Color::Rgb(109, 106, 128))
    }

    pub const fn select_description() -> Style {
        Style::new().fg(Color::Rgb(165, 163, 180))
    }

    pub const fn select_key() -> Style {
        Style::new().fg(Color::Rgb(160, 155, 180))
    }
}
