//! Logging and tracing configuration.
//!
//! Provides structured logging with different outputs for console and file.

use std::error::Error;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};

/// Initializes the tracing subscriber with console and file outputs.
/// 
/// # Features
/// - Console output: ANSI colors, no timestamps (except for "time" target)
/// - File output: No ANSI colors, timestamps for "time" target  
/// - Daily rolling log files in the `logs/` directory
/// 
/// # Returns
/// A guard that must be kept alive for the file appender to continue working.
pub fn init_tracing() -> Result<tracing_appender::non_blocking::WorkerGuard, Box<dyn Error>> {
    // Ensure logs directory exists
    std::fs::create_dir_all("logs")?;

    let file_appender = tracing_appender::rolling::daily("logs", "parser.log");
    let (file_nb, guard) = tracing_appender::non_blocking(file_appender);
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    // Terminal: no timestamp by default
    let stdout_no_ts = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout)
        .with_ansi(true)
        .with_target(false)
        .without_time()
        .with_filter(tracing_subscriber::filter::filter_fn(|meta| {
            meta.target() != "time"
        }));

    // Terminal: timestamp only for target "time"
    let stdout_ts = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout)
        .with_ansi(true)
        .with_target(false)
        .with_timer(tracing_subscriber::fmt::time::UtcTime::rfc_3339())
        .with_filter(tracing_subscriber::filter::filter_fn(|meta| {
            meta.target() == "time"
        }));

    // File: no ANSI, no timestamp by default
    let file_no_ts = tracing_subscriber::fmt::layer()
        .with_writer(file_nb.clone())
        .with_ansi(false)
        .with_target(false)
        .without_time()
        .with_filter(tracing_subscriber::filter::filter_fn(|meta| {
            meta.target() != "time"
        }));

    // File: timestamp only for target "time"
    let file_ts = tracing_subscriber::fmt::layer()
        .with_writer(file_nb)
        .with_ansi(false)
        .with_target(false)
        .with_timer(tracing_subscriber::fmt::time::UtcTime::rfc_3339())
        .with_filter(tracing_subscriber::filter::filter_fn(|meta| {
            meta.target() == "time"
        }));

    tracing_subscriber::registry()
        .with(filter)
        .with(stdout_no_ts)
        .with(stdout_ts)
        .with(file_no_ts)
        .with(file_ts)
        .try_init()?;
    Ok(guard)
}
