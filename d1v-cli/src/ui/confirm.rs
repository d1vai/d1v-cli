use super::widgets::{Answered, Canceled, Toggle};
use super::{Action, Terminal};
use crate::error::Error;
use crate::t;
use crossterm::event::KeyCode;

/// Yes/no confirmation prompt with toggle selector.
///
/// Uses Left/Right/Tab to switch between Yes and No, Enter to confirm.
pub struct Confirm {
    label: String,
    default: bool,
}

impl Confirm {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            default: false,
        }
    }

    #[must_use]
    pub fn default(mut self, default: bool) -> Self {
        self.default = default;
        self
    }

    /// Runs the prompt and returns the user's choice.
    pub fn prompt(self) -> Result<bool, Error> {
        let mut selected = self.default;
        let options = [t!("confirm-yes"), t!("confirm-no")];
        let mut term = Terminal::new(1)?;

        loop {
            let idx = usize::from(!selected);
            term.render(&Toggle::new(&self.label, [&options[0], &options[1]]).selected(idx))?;

            match Action::read()? {
                Some(Action::Submit) => {
                    term.commit(&Answered::new(&self.label, &options[idx]));
                    return Ok(selected);
                }
                Some(Action::Cancel) => {
                    term.commit(&Canceled::new(&self.label, &options[idx]));
                    return Err(Error::Canceled);
                }
                Some(Action::Input(key)) => match key.code {
                    KeyCode::Left | KeyCode::Right | KeyCode::Tab | KeyCode::BackTab => {
                        selected = !selected;
                    }
                    KeyCode::Char('y' | 'Y') => selected = true,
                    KeyCode::Char('n' | 'N') => selected = false,
                    _ => {}
                },
                None => {}
            }
        }
    }
}
