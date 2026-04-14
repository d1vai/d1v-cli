use std::future::Future;

use crossterm::terminal;
use rattles::presets::braille::Dots;
use rattles::TickedRattler;

use super::Terminal;
use crate::error::Error;

pub struct PendingPrompt {
    term: Terminal,
    label: String,
    display: String,
    value: String,
}

impl PendingPrompt {
    pub fn new(
        term: Terminal,
        label: impl Into<String>,
        display: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        Self {
            term,
            label: label.into(),
            display: display.into(),
            value: value.into(),
        }
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    /// Marks the prompt as answered and returns the collected value.
    pub fn commit(mut self) -> String {
        self.term.show_answered(&self.label, &self.display);
        self.value
    }

    /// Marks the prompt as canceled and discards the value.
    pub fn dismiss(mut self) {
        self.term.show_canceled(&self.label, &self.display);
    }

    /// Like [`spin`](Self::spin), but commits on `Ok` and dismisses on `Err`.
    pub async fn spin_ok<T>(
        self,
        task: impl Future<Output = Result<T, impl Into<Error>>>,
    ) -> Result<T, Error> {
        let (this, result) = self.spin(task).await?;
        match result {
            Ok(value) => {
                this.commit();
                Ok(value)
            }
            Err(err) => {
                this.dismiss();
                Err(err.into())
            }
        }
    }

    /// Animates a spinner while `task` runs, catching Ctrl+C as [`Error::Canceled`].
    pub async fn spin<T>(mut self, task: impl Future<Output = T>) -> Result<(Self, T), Error> {
        let _ = terminal::disable_raw_mode();

        // Extra line below spinner as visual "Enter pressed" feedback.
        let _ = self
            .term
            .set_viewport_height(2)
            .inspect_err(|err| tracing::debug!("failed to expand viewport: {err}"));

        let mut rattler = TickedRattler::<Dots>::new();
        let interval = rattler.interval();

        // Render the first frame immediately.
        self.term
            .show_pending(&self.label, &self.display, rattler.current_frame());
        rattler.tick();

        let mut timer = tokio::time::interval(interval);
        timer.tick().await;

        tokio::pin!(task);
        let interrupted = loop {
            tokio::select! {
                biased;
                result = &mut task => break Ok(result),
                _ = tokio::signal::ctrl_c() => break Err(()),
                _ = timer.tick() => {
                    self.term.show_pending(
                        &self.label,
                        &self.display,
                        rattler.current_frame(),
                    );
                    rattler.tick();
                }
            }
        };

        match interrupted {
            Ok(value) => Ok((self, value)),
            Err(()) => {
                self.dismiss();
                Err(Error::Canceled)
            }
        }
    }
}
