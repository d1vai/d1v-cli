use d1v_api::record::{Request, Response};
use d1v_api::Recorder;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::{fs, io};
use tracing::{info, warn};

/// A [`Recorder`] that buffers HTTP exchanges in memory and appends them
/// to a JSON file when dropped.
pub struct FileRecorder {
    path: PathBuf,
    exchanges: Mutex<Vec<Exchange>>,
}

#[derive(Serialize, Deserialize)]
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
        let pending = self.exchanges.get_mut();
        if pending.is_empty() {
            return;
        }

        if let Some(parent) = self.path.parent()
            && let Err(err) = std::fs::create_dir_all(parent)
        {
            warn!(%err, "failed to create recording directory");
            return;
        }

        let mut all = load_existing(&self.path);
        let new = pending.len();
        all.append(pending);

        match serde_json::to_string_pretty(&all) {
            Ok(json) => {
                if let Err(err) = std::fs::write(&self.path, json) {
                    warn!(%err, "failed to write recordings");
                } else {
                    info!(new, total = all.len(), path = %self.path.display(), "recorded HTTP exchanges");
                }
            }
            Err(err) => warn!(%err, "failed to serialize recordings"),
        }
    }
}

fn load_existing(path: impl AsRef<Path>) -> Vec<Exchange> {
    let path = path.as_ref();

    let data = match fs::read(path) {
        Ok(data) => data,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Vec::new(),
        Err(err) => {
            warn!(%err, "failed to read existing recordings");
            return Vec::new();
        }
    };

    match serde_json::from_slice::<Vec<Exchange>>(&data) {
        Ok(exchanges) => exchanges,
        Err(err) => {
            warn!(%err, "malformed recording file, backing up");
            let backup = path.with_added_extension("bak");

            if let Err(err) = fs::rename(path, &backup) {
                warn!(%err, "failed to back up malformed recording file");
            }

            Vec::new()
        }
    }
}
