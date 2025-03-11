use std::collections::VecDeque;
use std::fmt;
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
    entries: Arc<Mutex<VecDeque<LogEntry>>>,
    start_time: Instant,
}

impl Logger {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_LOG_ENTRIES))),
            start_time: Instant::now(),
        }
    }

    /// Add a message to the log with specified level
    pub fn log(&self, message: impl Into<String>, level: LogLevel) {
        let message = message.into();

        // Log to console as well for debugging
        match level {
            LogLevel::Error => eprintln!("[{}] {}", level, message),
            LogLevel::Warning => eprintln!("[{}] {}", level, message),
            _ => println!("[{}] {}", level, message),
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

    /// Helper method to log an info message
    pub fn info(&self, message: impl Into<String>) {
        self.log(message, LogLevel::Info);
    }

    /// Helper method to log a success message
    pub fn success(&self, message: impl Into<String>) {
        self.log(message, LogLevel::Success);
    }

    /// Helper method to log a warning message
    pub fn warning(&self, message: impl Into<String>) {
        self.log(message, LogLevel::Warning);
    }

    /// Helper method to log an error message
    pub fn error(&self, message: impl Into<String>) {
        self.log(message, LogLevel::Error);
    }

    /// Helper method to log a command message
    pub fn command(&self, message: impl Into<String>) {
        self.log(message, LogLevel::Command);
    }

    /// Helper method to log an output message
    pub fn output(&self, message: impl Into<String>) {
        self.log(message, LogLevel::Output);
    }

    /// Get all log entries for display
    pub fn get_entries(&self) -> Vec<LogEntry> {
        let entries = self.entries.lock().expect("Failed to lock logger mutex");
        entries.iter().cloned().collect()
    }

    /// Clear the log
    pub fn clear(&self) {
        let mut entries = self.entries.lock().expect("Failed to lock logger mutex");
        entries.clear();
    }

    /// Format a timestamp as relative time from start
    pub fn format_timestamp(&self, timestamp: Instant) -> String {
        let elapsed = timestamp.duration_since(self.start_time);
        format!("{:.2}s", elapsed.as_secs_f32())
    }
}

// Default implementation creates a new logger
impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}
