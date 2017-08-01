use std::time::Instant;
use std::io::Write;
use log::{set_logger, Log, LogLevel, LogLevelFilter, LogMetadata, LogRecord, MaxLogLevelFilter};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

pub fn init() -> Result<(), ::log::SetLoggerError> {
  set_logger(|max_level| {
    max_level.set(LogLevelFilter::Trace);
    Box::new(Logger::new(max_level))
  })
}

struct Logger {
  start_time: Instant,
  max_level: MaxLogLevelFilter,
}

impl Logger {
  pub fn new(max_level: MaxLogLevelFilter) -> Self {
    Logger {
      start_time: Instant::now(),
      max_level,
    }
  }

  fn filter(&self, record: &LogRecord) -> bool {
    let path = record.location().module_path();
    path.starts_with("scifiweb::") || path.starts_with("sf_")
  }
}

impl Log for Logger {
  fn enabled(&self, metadata: &LogMetadata) -> bool {
    self.max_level.get() >= metadata.level()
  }

  // If a log or color function fails, too bad...
  #[allow(unused_must_use)]
  fn log(&self, record: &LogRecord) {
    if !self.enabled(record.metadata()) || !self.filter(record) {
      return;
    }
    let mut stderr = StandardStream::stderr(ColorChoice::Always);
    let (color, title) = match record.metadata().level() {
      LogLevel::Debug => (Color::Green, "debug"),
      LogLevel::Trace => (Color::Blue, "trace"),
      LogLevel::Info => (Color::Cyan, " info"),
      LogLevel::Warn => (Color::Yellow, " warn"),
      LogLevel::Error => (Color::Red, "error"),
    };
    let mut color_spec = ColorSpec::new();
    stderr.set_color(color_spec.set_fg(Some(color.clone())).set_bold(true));
    write!(&mut stderr, "{} [", title);
    stderr.reset();
    write!(
      &mut stderr,
      "{}s",
      (Instant::now() - self.start_time).as_secs()
    );
    stderr.set_color(color_spec.set_fg(Some(color)).set_bold(true));
    write!(&mut stderr, "]:");
    stderr.reset();
    writeln!(
      &mut stderr,
      " ({}): {}",
      record.location().module_path(),
      record.args()
    );
  }
}
