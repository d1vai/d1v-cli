use std::fs::File;
use std::path::PathBuf;
use std::{fs, io};

use anyhow::Result;
use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};

use crate::config::Config;

/// Initializes stderr and file tracing.
pub fn init(log_file: Option<PathBuf>) -> Result<WorkerGuard> {
    let stderr_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));
    let stderr_layer = fmt::layer()
        .with_writer(io::stderr)
        .without_time()
        .with_filter(stderr_filter);

    let (non_blocking, guard) = file_writer(log_file)?;

    let file_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_filter(EnvFilter::new("d1v=debug,d1v_api=debug"));

    tracing_subscriber::registry()
        .with(stderr_layer)
        .with(file_layer)
        .try_init()
        .ok();

    Ok(guard)
}

/// Creates a non-blocking file writer for the log layer.
///
/// With `--log-file`, appends to the given path.
/// Otherwise, uses daily rotation under `~/.d1v/` (`d1v.YYYY-MM-DD.log`),
/// keeping the last 7 days.
fn file_writer(log_file: Option<PathBuf>) -> Result<(NonBlocking, WorkerGuard)> {
    match log_file {
        Some(path) => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }

            let file = File::options().create(true).append(true).open(&path)?;
            Ok(tracing_appender::non_blocking(file))
        }
        None => {
            let dir = Config::dir()?;
            fs::create_dir_all(&dir)?;

            let appender = RollingFileAppender::builder()
                .rotation(Rotation::DAILY)
                .filename_prefix("d1v")
                .filename_suffix("log")
                .max_log_files(8)
                .build(&dir)?;

            Ok(tracing_appender::non_blocking(appender))
        }
    }
}
