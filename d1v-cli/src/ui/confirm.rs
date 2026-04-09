use super::input::InputState;
use super::Terminal;
use crate::error::Error;
use crate::t;

/// Yes/no confirmation prompt with default value support.
///
/// Accepts `y`/`yes`/`n`/`no` (case-insensitive). Empty input uses the default
/// when set; invalid input shows an error and retries.
pub struct Confirm {
    label: String,
    default: Option<bool>,
    help: Option<String>,
}

impl Confirm {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            default: None,
            help: None,
        }
    }

    pub fn default(mut self, default: bool) -> Self {
        self.default = Some(default);
        self
    }

    pub fn help(mut self, msg: impl Into<String>) -> Self {
        self.help = Some(msg.into());
        self
    }

    fn hint(&self) -> &str {
        match self.default {
            Some(true) => "[Y/n]",
            Some(false) => "[y/N]",
            None => "[y/n]",
        }
    }

    /// Runs the prompt and returns the user's choice.
    pub fn prompt(self) -> Result<bool, Error> {
        let display_label = format!("{} {}", self.label, self.hint());

        let mut input = InputState::new();
        let mut error: Option<String> = None;
        let height = 1 + self.help.is_some() as u16;
        let mut term = Terminal::new(height)?;

        loop {
            let height = 1 + error.is_some() as u16 + self.help.is_some() as u16;
            term.set_viewport_height(height)?;

            loop {
                term.draw_prompt(
                    &display_label,
                    input.text(),
                    input.cursor_col(),
                    error.as_deref(),
                    self.help.as_deref(),
                )?;

                if term.read_key(&mut input, &self.label)? {
                    break;
                }
            }

            let text = input.text().trim();
            let result = if text.is_empty() {
                self.default
            } else {
                parse_bool(text)
            };

            if let Some(value) = result {
                let answer = format_bool(value);
                term.show_answered(&self.label, &answer);
                return Ok(value);
            }

            error = Some(t!("confirm-invalid"));
            input.clear();
        }
    }
}

fn parse_bool(input: &str) -> Option<bool> {
    match input.to_lowercase().as_str() {
        "y" | "yes" => Some(true),
        "n" | "no" => Some(false),
        _ => None,
    }
}

fn format_bool(value: bool) -> String {
    if value {
        t!("confirm-yes")
    } else {
        t!("confirm-no")
    }
}
