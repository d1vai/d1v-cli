use std::fs::File;
use std::path::PathBuf;
use std::{fs, io};

use anyhow::Result;
use tracing_appender::non_blocking::WorkerGuard;
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

    let path = log_file.unwrap_or(Config::dir()?.join("d1v.log"));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let file = File::create(&path)?;
    let (non_blocking, guard) = tracing_appender::non_blocking(file);

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
