use std::collections::VecDeque;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

// Maximum entries to prevent unbounded memory growth
const MAX_LOG_ENTRIES: usize = 500;

/// A single log entry with timestamp and message
#[derive(Clone, Debug)]
pub struct LogEntry {
    pub timestamp: Instant,
    pub message: String,
    pub level: LogLevel,
}

/// Log levels for different types of messages
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Success,
    Error,
    Command,
    Output,
    Warning,
}

// Implement Display for LogLevel to easily format the level prefixes
impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Warning => write!(f, "WARN"),
            LogLevel::Success => write!(f, "SUCCESS"),
            LogLevel::Command => write!(f, "CMD"),
            LogLevel::Output => write!(f, "OUTPUT"),
            LogLevel::Info => write!(f, "INFO"),
        }
    }
}

/// The logger itself, wrapped in Arc<Mutex<>> for thread safety
#[derive(Clone)]
pub struct Logger {
    name: String,
    enabled: bool,
    debug_mode: Arc<AtomicBool>,
    entries: Arc<Mutex<VecDeque<LogEntry>>>,
    start_time: Instant,
}

impl Logger {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            enabled: true,
            debug_mode: Arc::new(AtomicBool::new(false)),
            entries: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_LOG_ENTRIES))),
            start_time: Instant::now(),
        }
    }

    /// Add a message to the log with specified level
    pub fn log(&self, message: impl Into<String>, level: LogLevel) {
        // Skip logging if logger is disabled
        if !self.enabled {
            return;
        }

        let message = message.into();

        // Log to console with logger name
        match level {
            LogLevel::Error => eprintln!("[{}][{}] {}", self.name, level, message),
            LogLevel::Warning => eprintln!("[{}][{}] {}", self.name, level, message),
            _ => println!("[{}][{}] {}", self.name, level, message),
        }

        let mut entries = self.entries.lock().expect("Failed to lock logger mutex");

        // Add the new entry
        entries.push_back(LogEntry {
            timestamp: Instant::now(),
            message,
            level,
        });

        // Ensure we don't exceed max capacity
        if entries.len() > MAX_LOG_ENTRIES {
            entries.pop_front();
        }
    }

    pub fn info(&self, message: impl Into<String>) {
        self.log(message, LogLevel::Info);
    }

    pub fn success(&self, message: impl Into<String>) {
        self.log(message, LogLevel::Success);
    }

    pub fn warning(&self, message: impl Into<String>) {
        self.log(message, LogLevel::Warning);
    }

    pub fn error(&self, message: impl Into<String>) {
        self.log(message, LogLevel::Error);
    }

    pub fn command(&self, message: impl Into<String>) {
        self.log(message, LogLevel::Command);
    }

    pub fn output(&self, message: impl Into<String>) {
        self.log(message, LogLevel::Output);
    }

    pub fn get_entries(&self) -> Vec<LogEntry> {
        let entries = self.entries.lock().expect("Failed to lock logger mutex");
        entries.iter().cloned().collect()
    }

    pub fn clear(&self) {
        let mut entries = self.entries.lock().expect("Failed to lock logger mutex");
        entries.clear();
    }

    /// Format a timestamp as relative time from start
    pub fn format_timestamp(&self, timestamp: Instant) -> String {
        let elapsed = timestamp.duration_since(self.start_time);
        format!("{:.2}s", elapsed.as_secs_f32())
    }

    #[cfg(debug_assertions)]
    pub fn set_debug_mode(&self, debug: bool) {
        self.debug_mode.store(debug, Ordering::SeqCst);
    }

    /// Debug-level logging — only outputs when debug mode is enabled.
    pub fn debug(&self, message: impl Into<String>) {
        // Only log if debug mode is enabled AND logger is enabled
        if self.enabled && self.debug_mode.load(Ordering::SeqCst) {
            self.log(message, LogLevel::Info);
        }
    }
}

// Default implementation creates a new logger
impl Default for Logger {
    fn default() -> Self {
        Self::new("DefaultLogger")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_logger_starts_empty() {
        let logger = Logger::new("Test");
        assert!(logger.get_entries().is_empty());
    }

    #[test]
    fn logs_are_recorded() {
        let logger = Logger::new("Test");
        logger.info("Hello");
        logger.error("Oops");
        logger.warning("Careful");
        assert_eq!(logger.get_entries().len(), 3);
    }

    #[test]
    fn clear_removes_all() {
        let logger = Logger::new("Test");
        logger.info("1");
        logger.info("2");
        logger.clear();
        assert!(logger.get_entries().is_empty());
    }

    #[test]
    fn levels_are_correct() {
        let logger = Logger::new("Test");
        logger.info("a");
        logger.error("b");
        logger.success("c");
        logger.warning("d");
        logger.command("e");
        logger.output("f");

        let entries = logger.get_entries();
        assert_eq!(entries[0].level, LogLevel::Info);
        assert_eq!(entries[1].level, LogLevel::Error);
        assert_eq!(entries[2].level, LogLevel::Success);
        assert_eq!(entries[3].level, LogLevel::Warning);
        assert_eq!(entries[4].level, LogLevel::Command);
        assert_eq!(entries[5].level, LogLevel::Output);
    }

    #[test]
    fn message_preserved() {
        let logger = Logger::new("Test");
        logger.info("exact content 123");
        assert_eq!(logger.get_entries()[0].message, "exact content 123");
    }

    #[test]
    fn clone_shares_entries() {
        let a = Logger::new("Shared");
        let b = a.clone();
        a.info("from a");
        b.info("from b");
        assert_eq!(a.get_entries().len(), 2);
        assert_eq!(b.get_entries().len(), 2);
    }

    #[test]
    fn respects_max_capacity() {
        let logger = Logger::new("Cap");
        for i in 0..600 {
            logger.info(format!("msg-{i}"));
        }
        let entries = logger.get_entries();
        assert!(entries.len() <= MAX_LOG_ENTRIES);
        // Oldest entries evicted — first entry should NOT be msg-0
        assert!(!entries[0].message.contains("msg-0"));
    }

    #[test]
    fn debug_suppressed_by_default() {
        let logger = Logger::new("Test");
        logger.debug("invisible");
        assert!(logger.get_entries().is_empty());
    }

    #[test]
    fn debug_visible_when_enabled() {
        let logger = Logger::new("Test");
        logger.set_debug_mode(true);
        logger.debug("visible");
        assert_eq!(logger.get_entries().len(), 1);
        assert_eq!(logger.get_entries()[0].message, "visible");
    }

    #[test]
    fn timestamp_format_ends_with_s() {
        let logger = Logger::new("Ts");
        logger.info("t");
        let ts = logger.format_timestamp(logger.get_entries()[0].timestamp);
        assert!(ts.ends_with('s'), "Got: {ts}");
    }

    #[test]
    fn thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let logger = Arc::new(Logger::new("Thread"));
        let mut handles = vec![];

        for i in 0..10 {
            let log = Arc::clone(&logger);
            handles.push(thread::spawn(move || {
                for j in 0..50 {
                    log.info(format!("t{i}-{j}"));
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(logger.get_entries().len(), 500);
    }

    #[test]
    fn loglevel_display() {
        assert_eq!(format!("{}", LogLevel::Error), "ERROR");
        assert_eq!(format!("{}", LogLevel::Warning), "WARN");
        assert_eq!(format!("{}", LogLevel::Info), "INFO");
        assert_eq!(format!("{}", LogLevel::Success), "SUCCESS");
        assert_eq!(format!("{}", LogLevel::Command), "CMD");
        assert_eq!(format!("{}", LogLevel::Output), "OUTPUT");
    }
}
