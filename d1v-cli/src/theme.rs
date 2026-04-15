pub mod owo {
    use owo_colors::Style;

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
}
