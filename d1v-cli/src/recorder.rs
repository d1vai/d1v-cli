use d1v_api::record::{Request, Response};
use d1v_api::Recorder;
use parking_lot::Mutex;
use serde::Serialize;
use std::path::PathBuf;
use tracing::warn;

/// A [`Recorder`] that collects HTTP exchanges in memory and writes them
/// to a single JSON file on drop.
pub struct FileRecorder {
    path: PathBuf,
    exchanges: Mutex<Vec<Exchange>>,
}

#[derive(Serialize)]
struct Exchange {
    request: Request,
    response: Response,
}

impl FileRecorder {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            exchanges: Mutex::new(Vec::new()),
        }
    }
}

impl Recorder for FileRecorder {
    fn record(&self, request: &Request, response: &Response) {
        self.exchanges.lock().push(Exchange {
            request: request.clone(),
            response: response.clone(),
        });
    }
}

impl Drop for FileRecorder {
    fn drop(&mut self) {
        let exchanges = self.exchanges.get_mut();
        if exchanges.is_empty() {
            return;
        }

        if let Some(parent) = self.path.parent()
            && let Err(err) = std::fs::create_dir_all(parent)
        {
            warn!(%err, "failed to create recording directory");
            return;
        }

        match serde_json::to_string_pretty(exchanges) {
            Ok(json) => {
                if let Err(err) = std::fs::write(&self.path, json) {
                    warn!(%err, "failed to write recordings");
                }
            }
            Err(err) => warn!(%err, "failed to serialize recordings"),
        }
    }
}
