use std::process;

use crossterm::event::{self, KeyCode, KeyModifiers};

use super::{ctrl_c_hint_line, nav_hint_line, SelectItem, Terminal};
use crate::error::Error;

/// A single choice in a [`Select`] prompt.
pub struct SelectOption<T> {
    label: String,
    description: Option<String>,
    /// The value returned when this option is chosen.
    pub value: T,
}

impl<T> SelectOption<T> {
    pub fn new(value: T, label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: None,
            value,
        }
    }

    #[must_use]
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Key press classified as a select action.
#[derive(Debug, Clone)]
enum SelectAction {
    /// Move selection up (↑ / k / Shift+Tab).
    Up,
    /// Move selection down (↓ / j / Tab).
    Down,
    /// Jump directly to an option by 1-based number key.
    Jump(usize),
    /// Confirm the current selection (Enter).
    Submit,
    /// Cancel immediately (Esc).
    Cancel,
    /// Ctrl+C pressed.
    CtrlC,
    /// Unhandled key — resets any pending state.
    Other,
}

impl SelectAction {
    /// Reads and classifies one key event.
    ///
    /// Returns `None` for non-key-press events (release, repeat).
    /// Numeric keys beyond `n` options are classified as [`Other`](Self::Other).
    fn read(n: usize) -> std::io::Result<Option<Self>> {
        let Some(key) = event::read()?.as_key_press_event() else {
            return Ok(None);
        };

        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        Ok(Some(match key.code {
            KeyCode::Enter => Self::Submit,
            KeyCode::Esc => Self::Cancel,
            KeyCode::Char('c') if ctrl => Self::CtrlC,
            KeyCode::Up | KeyCode::BackTab => Self::Up,
            KeyCode::Char('k') if !ctrl => Self::Up,
            KeyCode::Down | KeyCode::Tab => Self::Down,
            KeyCode::Char('j') if !ctrl => Self::Down,
            KeyCode::Char(c @ '1'..='9') if !ctrl => {
                let idx = (c as usize) - ('1' as usize);
                if idx < n {
                    Self::Jump(idx)
                } else {
                    Self::Other
                }
            }
            _ => Self::Other,
        }))
    }
}

/// Event loop state for [`Select`].
struct State {
    selected: usize,
    exit_pending: bool,
}

/// Outcome of handling a [`SelectAction`].
enum Outcome {
    /// User confirmed selection.
    Submit,
    /// User pressed Esc.
    Cancel,
    /// Double Ctrl+C — force exit process.
    ForceExit,
}

impl State {
    fn new(n: usize, default: Option<usize>) -> Self {
        Self {
            selected: default.unwrap_or(0).min(n - 1),
            exit_pending: false,
        }
    }

    /// Processes an action. Returns `Some` if the prompt should end.
    fn handle(&mut self, action: SelectAction, n: usize) -> Option<Outcome> {
        match action {
            SelectAction::Submit => Some(Outcome::Submit),
            SelectAction::Cancel => Some(Outcome::Cancel),
            SelectAction::CtrlC => {
                if self.exit_pending {
                    Some(Outcome::ForceExit)
                } else {
                    self.exit_pending = true;
                    None
                }
            }
            SelectAction::Up => {
                self.exit_pending = false;
                self.selected = if self.selected == 0 {
                    n - 1
                } else {
                    self.selected - 1
                };
                None
            }
            SelectAction::Down => {
                self.exit_pending = false;
                self.selected = (self.selected + 1) % n;
                None
            }
            SelectAction::Jump(idx) => {
                self.exit_pending = false;
                self.selected = idx;
                None
            }
            SelectAction::Other => {
                self.exit_pending = false;
                None
            }
        }
    }
}

/// Vertical list selector prompt.
pub struct Select<T> {
    label: String,
    options: Vec<SelectOption<T>>,
    default: Option<usize>,
}

impl<T> Select<T> {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            options: Vec::new(),
            default: None,
        }
    }

    #[must_use]
    pub fn option(mut self, opt: SelectOption<T>) -> Self {
        self.options.push(opt);
        self
    }

    #[must_use]
    pub fn options(mut self, opts: impl IntoIterator<Item = SelectOption<T>>) -> Self {
        self.options.extend(opts);
        self
    }

    #[must_use]
    pub fn default_index(mut self, idx: usize) -> Self {
        self.default = Some(idx);
        self
    }

    /// Runs the interactive select loop and returns the chosen value.
    ///
    /// Returns `Err(Error::Canceled)` if the user cancels with Esc.
    /// Silently exits the process on double Ctrl+C.
    pub fn prompt(mut self) -> Result<T, Error> {
        let n = self.options.len();
        assert!(n > 0, "Select must have at least one option");

        let mut state = State::new(n, self.default);

        // Snapshot labels so the render loop can borrow them independently of `options`.
        let display: Vec<(String, Option<String>)> = self
            .options
            .iter()
            .map(|option| (option.label.clone(), option.description.clone()))
            .collect();

        let mut term = Terminal::new(n as u16 + 4)?;

        loop {
            let hint = if state.exit_pending {
                ctrl_c_hint_line()
            } else {
                nav_hint_line()
            };

            let items: Vec<SelectItem<'_>> = display
                .iter()
                .map(|(label, desc)| SelectItem {
                    label: label.as_str(),
                    description: desc.as_deref(),
                })
                .collect();

            term.draw_select(&self.label, &items, state.selected, hint)?;

            let Some(action) = SelectAction::read(n)? else {
                continue;
            };

            match state.handle(action, n) {
                Some(Outcome::Submit) => {
                    let option = self.options.remove(state.selected);
                    term.show_answered(&self.label, &display[state.selected].0);
                    return Ok(option.value);
                }
                Some(Outcome::Cancel) => {
                    term.show_canceled(&self.label, &display[state.selected].0);
                    return Err(Error::Canceled);
                }
                Some(Outcome::ForceExit) => {
                    drop(term);
                    process::exit(0);
                }
                None => {}
            }
        }
    }
}
