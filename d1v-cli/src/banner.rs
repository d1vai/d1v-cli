use std::fmt::Write;
use std::sync::LazyLock;

use colorgrad::{Color, Gradient, GradientBuilder, LinearGradient};
use itertools::Itertools;
use owo_colors::{OwoColorize, Stream};

static GRADIENT: LazyLock<LinearGradient> = LazyLock::new(|| {
    GradientBuilder::new()
        .colors(&[
            Color::from_rgba8(0x4f, 0x46, 0xe5, 0xff),
            Color::from_rgba8(0x8b, 0x5c, 0xf6, 0xff),
            Color::from_rgba8(0xc0, 0x26, 0xd3, 0xff),
            Color::from_rgba8(0xf4, 0x3f, 0x5e, 0xff),
        ])
        .build()
        .expect("valid gradient colors")
});

const ROWS: usize = 6;

// ANSI Shadow figlet glyphs. Face uses ██, shadow uses ╔═╗║╚╝.
type Letter = [&'static str; ROWS];

#[rustfmt::skip]
const D: Letter = [
    "██████╗ ",
    "██╔══██╗",
    "██║  ██║",
    "██║  ██║",
    "██████╔╝",
    "╚═════╝ ",
];

#[rustfmt::skip]
const ONE: Letter = [
    " ██╗",
    "███║",
    "╚██║",
    " ██║",
    " ██║",
    " ╚═╝",
];

#[rustfmt::skip]
const V: Letter = [
    "██╗   ██╗",
    "██║   ██║",
    "██║   ██║",
    "╚██╗ ██╔╝",
    " ╚████╔╝ ",
    "  ╚═══╝  ",
];

#[rustfmt::skip]
const C: Letter = [
    " ██████╗",
    "██╔════╝",
    "██║     ",
    "██║     ",
    "╚██████╗",
    " ╚═════╝",
];

#[rustfmt::skip]
const L: Letter = [
    "██╗     ",
    "██║     ",
    "██║     ",
    "██║     ",
    "███████╗",
    "╚══════╝",
];

#[rustfmt::skip]
const I: Letter = [
    "██╗",
    "██║",
    "██║",
    "██║",
    "██║",
    "╚═╝",
];

pub struct Banner {
    pub padding_top: &'static str,
    pub padding_bottom: &'static str,
    pub padding_left: &'static str,
    pub letter_spacing: &'static str,
    pub word_spacing: &'static str,
    pub shadow_dim: f32,
}

impl Default for Banner {
    fn default() -> Self {
        Self {
            padding_top: "\n\n",
            padding_bottom: "\n\n",
            padding_left: "  ",
            letter_spacing: " ",
            word_spacing: "     ",
            shadow_dim: 0.45,
        }
    }
}

impl Banner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn padding_top(mut self, padding_top: &'static str) -> Self {
        self.padding_top = padding_top;
        self
    }

    pub fn padding_bottom(mut self, padding_bottom: &'static str) -> Self {
        self.padding_bottom = padding_bottom;
        self
    }

    pub fn padding_left(mut self, padding_left: &'static str) -> Self {
        self.padding_left = padding_left;
        self
    }

    pub fn letter_spacing(mut self, letter_spacing: &'static str) -> Self {
        self.letter_spacing = letter_spacing;
        self
    }

    pub fn word_spacing(mut self, word_spacing: &'static str) -> Self {
        self.word_spacing = word_spacing;
        self
    }

    pub fn shadow_dim(mut self, shadow_dim: f32) -> Self {
        self.shadow_dim = shadow_dim;
        self
    }

    /// Composes letter glyphs into plain-text rows.
    fn compose(&self) -> Vec<String> {
        let words: &[&[&Letter]] = &[&[&D, &ONE, &V], &[&C, &L, &I]];

        (0..ROWS)
            .map(|row| {
                words
                    .iter()
                    .map(|word| {
                        word.iter()
                            .map(|letter| letter[row])
                            .join(self.letter_spacing)
                    })
                    .join(self.word_spacing)
            })
            .collect()
    }

    fn is_shadow(ch: char) -> bool {
        matches!(ch, '╗' | '╔' | '║' | '═' | '╚' | '╝')
    }

    fn dim(&self, r: u8, g: u8, b: u8) -> (u8, u8, u8) {
        (
            (f32::from(r) * self.shadow_dim) as u8,
            (f32::from(g) * self.shadow_dim) as u8,
            (f32::from(b) * self.shadow_dim) as u8,
        )
    }

    /// Applies gradient coloring to a single composed row, dimming shadow characters.
    fn colorize_line(&self, line: &str, colors: &[Color]) -> String {
        let mut out = String::from(self.padding_left);

        for (col, ch) in line.chars().enumerate() {
            if ch == ' ' {
                out.push(' ');
                continue;
            }

            let [r, g, b, _] = colors[col].to_rgba8();
            let (r, g, b) = if Self::is_shadow(ch) {
                self.dim(r, g, b)
            } else {
                (r, g, b)
            };

            let _ = write!(
                out,
                "{}",
                ch.if_supports_color(Stream::Stdout, |c| c.truecolor(r, g, b))
            );
        }

        out
    }

    /// Renders the "D1V CLI" ASCII art banner with gradient and shadow.
    pub fn render(&self) -> String {
        let lines = self.compose();
        let width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(1);
        let colors = GRADIENT.colors(width);

        let art = lines
            .iter()
            .map(|line| self.colorize_line(line, &colors))
            .join("\n");

        format!("{}{art}{}", self.padding_top, self.padding_bottom)
    }
}
