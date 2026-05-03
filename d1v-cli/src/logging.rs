use std::fs::File;
use std::path::PathBuf;
use std::{fs, io};

use crate::error::Result;
use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{EnvFilter, fmt};

use crate::config::Config;

/// Initializes stderr and file tracing.
///
/// `verbose` controls log verbosity on both stderr and file layers:
/// - 0: stderr `warn`, file `debug`
/// - 1: stderr `info`, file `debug`
/// - 2: stderr `debug`, file `debug`
/// - 3+: stderr `trace`, file `trace`
pub fn init(log_file: Option<PathBuf>, verbose: u8) -> Result<WorkerGuard> {
    let stderr_layer = fmt::layer()
        .with_writer(io::stderr)
        .without_time()
        .with_filter(stderr_filter(verbose));

    let (non_blocking, guard) = file_writer(log_file)?;

    let file_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_filter(file_filter(verbose));

    tracing_subscriber::registry()
        .with(stderr_layer)
        .with(file_layer)
        .try_init()
        .ok();

    Ok(guard)
}

fn stderr_filter(verbose: u8) -> EnvFilter {
    match verbose {
        0 => EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        1 => EnvFilter::new("d1v=info,d1v_api=info"),
        2 => EnvFilter::new("d1v=debug,d1v_api=debug"),
        _ => EnvFilter::new("trace"),
    }
}

fn file_filter(verbose: u8) -> EnvFilter {
    EnvFilter::new(if verbose >= 3 {
        "trace"
    } else {
        "d1v=debug,d1v_api=debug"
    })
}

/// Creates a non-blocking file writer for the log layer.
///
/// With `--log-file`, appends to the given path.
/// Otherwise, uses daily rotation under `~/.d1v/` (`d1v.YYYY-MM-DD.log`),
/// keeping the last 7 days.
fn file_writer(log_file: Option<PathBuf>) -> Result<(NonBlocking, WorkerGuard)> {
    if let Some(path) = log_file {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let file = File::options().create(true).append(true).open(&path)?;
        Ok(tracing_appender::non_blocking(file))
    } else {
        let dir = Config::dir()?;
        fs::create_dir_all(&dir)?;

        let appender = RollingFileAppender::builder()
            .rotation(Rotation::DAILY)
            .filename_prefix("d1v")
            .filename_suffix("log")
            .max_log_files(8)
            .build(&dir)
            .map_err(anyhow::Error::from)?;

        Ok(tracing_appender::non_blocking(appender))
    }
}
