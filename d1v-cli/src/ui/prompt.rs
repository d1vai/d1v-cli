use std::future::Future;

use crossterm::terminal;
use rattles::presets::braille::Dots;
use rattles::TickedRattler;

use super::Terminal;

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

    pub fn commit(mut self) -> String {
        self.term.show_answered(&self.label, &self.display);
        self.value
    }

    pub fn dismiss(mut self) {
        self.term.show_canceled(&self.label, &self.display);
    }

    pub async fn spin<T>(mut self, task: impl Future<Output = T>) -> (Self, T) {
        let _ = terminal::disable_raw_mode();

        let mut rattler = TickedRattler::<Dots>::new();
        let interval = rattler.interval();

        // Render the first frame immediately.
        self.term
            .show_pending(&self.label, &self.display, rattler.current_frame());
        rattler.tick();

        let mut timer = tokio::time::interval(interval);
        timer.tick().await;

        tokio::pin!(task);
        let result = loop {
            tokio::select! {
                biased;
                result = &mut task => break result,
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

        (self, result)
    }
}
