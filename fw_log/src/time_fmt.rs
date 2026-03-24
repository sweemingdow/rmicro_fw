use tracing_subscriber::fmt;
use tracing_subscriber::fmt::time::FormatTime;

pub struct ConsoleTimer;

impl FormatTime for ConsoleTimer {
    fn format_time(&self, w: &mut fmt::format::Writer<'_>) -> std::fmt::Result {
        _fmt_now(w, "%Y-%m-%d %H:%M:%S%.3f")
    }
}

pub struct ProgramTimer;

impl FormatTime for ProgramTimer {
    fn format_time(&self, w: &mut fmt::format::Writer<'_>) -> std::fmt::Result {
        _fmt_now(w, "%Y-%m-%dT%H:%M:%S%.3f%:z")
    }
}

fn _fmt_now(w: &mut fmt::format::Writer<'_>, fmt: &str) -> std::fmt::Result {
    let now = chrono::Local::now();
    write!(w, "{}", now.format(fmt))
}
