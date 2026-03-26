use crate::time_fmt;
use tracing_appender::non_blocking;
use tracing_appender::rolling;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

pub struct LogOptions {
    pub max_log_files: u16,
    pub log_dir: String,
    pub app_name: String,
    pub port: u16,
    pub thread_name: bool,
    pub thread_id: bool,
}

pub fn init_logger(log_ops: LogOptions) -> non_blocking::WorkerGuard {
    // 控制台日志
    let console_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_thread_ids(log_ops.thread_id)
        .with_thread_names(log_ops.thread_name)
        .with_file(true)
        .with_line_number(true)
        .with_target(true)
        .with_timer(time_fmt::ConsoleTimer);

    let rolling_appender = rolling::RollingFileAppender::builder()
        .rotation(rolling::Rotation::DAILY)
        .filename_suffix("log")
        .max_log_files(log_ops.max_log_files as usize)
        .build(format!(
            "{}/{}/{}",
            log_ops.log_dir, log_ops.app_name, log_ops.port
        ))
        .expect("init rolling file appender failed");

    let (file_writer, _guard) = tracing_appender::non_blocking(rolling_appender);

    let file_layer = fmt::layer()
        .json()
        .with_writer(file_writer)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .with_target(true)
        .with_timer(time_fmt::ProgramTimer)
        .with_span_list(false);

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));

    tracing_subscriber::registry()
        .with(filter)
        .with(console_layer)
        .with(file_layer)
        .init();

    _guard
}
