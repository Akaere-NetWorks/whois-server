// WHOIS Server - Systemd-Style Logger
// Copyright (C) 2025 Akaere Networks
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Systemd-style logging implementation compatible with journald
//!
//! This logger provides structured logging that follows systemd/journald conventions:
//! - Log levels: emerg, alert, crit, err, warning, notice, info, debug
//! - Structured fields with proper prefixes
//! - Clean, readable output for both terminal and journald
//! - Thread-safe async-friendly implementation

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Log levels following systemd priority conventions
/// https://www.freedesktop.org/software/systemd/man/sd-daemon.html
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
#[allow(dead_code)] // Some log levels are reserved for future use
pub enum LogLevel {
    /// System is unusable (0)
    Emergency = 0,
    /// Action must be taken immediately (1)
    Alert = 1,
    /// Critical conditions (2)
    Critical = 2,
    /// Error conditions (3)
    Error = 3,
    /// Warning conditions (4)
    Warning = 4,
    /// Normal but significant condition (5)
    Notice = 5,
    /// Informational message (6)
    Info = 6,
    /// Debug-level message (7)
    Debug = 7,
}

impl LogLevel {
    /// Convert numeric priority to LogLevel
    #[allow(dead_code)] // Reserved for future use
    pub fn from_priority(priority: u8) -> Self {
        match priority {
            0 => LogLevel::Emergency,
            1 => LogLevel::Alert,
            2 => LogLevel::Critical,
            3 => LogLevel::Error,
            4 => LogLevel::Warning,
            5 => LogLevel::Notice,
            6 => LogLevel::Info,
            7 => LogLevel::Debug,
            _ => LogLevel::Info, // Default to info for unknown values
        }
    }

    /// Get the priority number for systemd
    pub fn priority(self) -> u8 {
        self as u8
    }

    /// Get the string representation
    pub fn as_str(self) -> &'static str {
        match self {
            LogLevel::Emergency => "EMERG",
            LogLevel::Alert => "ALERT",
            LogLevel::Critical => "CRIT",
            LogLevel::Error => "ERR",
            LogLevel::Warning => "WARNING",
            LogLevel::Notice => "NOTICE",
            LogLevel::Info => "INFO",
            LogLevel::Debug => "DEBUG",
        }
    }

    /// Get color code for terminal output
    pub fn color_code(self) -> &'static str {
        match self {
            LogLevel::Emergency => "\x1b[1;41m", // Bold red background
            LogLevel::Alert => "\x1b[1;91m",     // Bold bright red
            LogLevel::Critical => "\x1b[1;31m",  // Bold red
            LogLevel::Error => "\x1b[31m",       // Red
            LogLevel::Warning => "\x1b[33m",     // Yellow
            LogLevel::Notice => "\x1b[36m",      // Cyan
            LogLevel::Info => "\x1b[32m",        // Green
            LogLevel::Debug => "\x1b[37m",       // White/gray
        }
    }
}

/// Logger configuration
#[derive(Debug, Clone)]
pub struct LoggerConfig {
    /// Minimum log level to output
    pub min_level: LogLevel,
    /// Whether to use colors in output
    pub use_colors: bool,
    /// Whether to include timestamps
    pub include_timestamp: bool,
    /// Whether to include target/module information
    pub include_target: bool,
    /// Whether to format for journald (structured format)
    pub journald_format: bool,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            min_level: LogLevel::Info,
            use_colors: atty::is(atty::Stream::Stderr),
            include_timestamp: true,
            include_target: false,
            journald_format: false,
        }
    }
}

/// Global logger instance
static LOGGER: Mutex<Option<Logger>> = Mutex::new(None);

/// Systemd-style logger implementation
#[derive(Debug)]
pub struct Logger {
    config: LoggerConfig,
    min_level: AtomicU8,
}

impl Logger {
    /// Create a new logger with the given configuration
    pub fn new(config: LoggerConfig) -> Self {
        Self {
            min_level: AtomicU8::new(config.min_level.priority()),
            config,
        }
    }

    /// Initialize the global logger
    pub fn init(config: LoggerConfig) -> Result<(), LoggerError> {
        let logger = Self::new(config);

        // Store logger in global static
        {
            let mut global_logger = LOGGER.lock().map_err(|_| LoggerError::InitError)?;
            if global_logger.is_some() {
                return Err(LoggerError::AlreadyInitialized);
            }
            *global_logger = Some(logger);
        }

        Ok(())
    }

    /// Set the minimum log level at runtime
    #[allow(dead_code)] // Reserved for future use
    pub fn set_min_level(&self, level: LogLevel) {
        self.min_level.store(level.priority(), Ordering::Relaxed);
    }

    /// Check if a log level should be output
    pub fn should_log(&self, level: LogLevel) -> bool {
        level.priority() <= self.min_level.load(Ordering::Relaxed)
    }

    /// Log a message with the given level and fields
    pub fn log(&self, level: LogLevel, target: &str, message: &str) {
        if !self.should_log(level) {
            return;
        }

        let timestamp = if self.config.include_timestamp {
            Some(SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs())
        } else {
            None
        };

        let formatted = if self.config.journald_format {
            self.format_journald(level, target, message, timestamp)
        } else {
            self.format_terminal(level, target, message, timestamp)
        };

        eprintln!("{}", formatted);
    }

    /// Format for journald structured output
    fn format_journald(&self, level: LogLevel, target: &str, message: &str, timestamp: Option<u64>) -> String {
        let mut output = String::new();

        // Priority field for journald
        output.push_str(&format!("PRIORITY={}\n", level.priority()));

        // Message field
        output.push_str(&format!("MESSAGE={}\n", message));

        // Add target if specified
        if self.config.include_target && !target.is_empty() {
            output.push_str(&format!("CODE_FILE={}\n", target));
        }

        // Add timestamp if available
        if let Some(ts) = timestamp {
            output.push_str(&format!("_SOURCE_REALTIME_TIMESTAMP={}\n", ts * 1_000_000)); // microseconds
        }

        // Add our service identifier
        output.push_str("SYSLOG_IDENTIFIER=whois-server\n");

        output
    }

    /// Format for terminal output
    fn format_terminal(&self, level: LogLevel, _target: &str, message: &str, timestamp: Option<u64>) -> String {
        let mut output = String::new();

        // Timestamp
        if let Some(ts) = timestamp {
            let datetime = chrono::DateTime::from_timestamp(ts as i64, 0)
                .unwrap_or_default()
                .format("%Y-%m-%d %H:%M:%S");
            output.push_str(&format!("{} ", datetime));
        }

        // Check if message already has [..] format (systemd-style)
        if message.starts_with('[') && (message.contains("[*]") || message.contains("[   OK   ]") || message.contains("[  FAILED ]") || message.contains("[   WARN ]") || message.contains("[ INFO ]") || message.contains("[ DEBUG ]")) {
            // For systemd-style messages, don't add level prefix, just color the status
            if self.config.use_colors {
                if message.contains("[  FAILED ]") {
                    output.push_str(&format!(
                        "{}{}{}\x1b[0m",
                        LogLevel::Error.color_code(),
                        message,
                        "\x1b[0m"
                    ));
                } else if message.contains("[   WARN ]") {
                    output.push_str(&format!(
                        "{}{}{}\x1b[0m",
                        LogLevel::Warning.color_code(),
                        message,
                        "\x1b[0m"
                    ));
                } else if message.contains("[   OK   ]") {
                    output.push_str(&format!(
                        "{}{}{}\x1b[0m",
                        LogLevel::Info.color_code(),
                        message,
                        "\x1b[0m"
                    ));
                } else if message.contains("[ INFO ]") {
                    output.push_str(&format!(
                        "{}{}{}\x1b[0m",
                        LogLevel::Info.color_code(),
                        message,
                        "\x1b[0m"
                    ));
                } else if message.contains("[ DEBUG ]") {
                    output.push_str(&format!(
                        "{}{}{}\x1b[0m",
                        LogLevel::Debug.color_code(),
                        message,
                        "\x1b[0m"
                    ));
                } else {
                    output.push_str(message);
                }
            } else {
                output.push_str(message);
            }
        } else {
            // For regular messages, add level in brackets
            if self.config.use_colors {
                output.push_str(&format!(
                    "{}[{}]{}\x1b[0m {}",
                    level.color_code(),
                    level.as_str(),
                    "\x1b[0m",
                    message
                ));
            } else {
                output.push_str(&format!("[{}] {}", level.as_str(), message));
            }
        }

        output
    }
}

/// Convenience macros for logging
#[macro_export]
macro_rules! log_emerg {
    ($($arg:tt)*) => {
        $crate::core::logger::log_with_level($crate::core::logger::LogLevel::Emergency, module_path!(), &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_alert {
    ($($arg:tt)*) => {
        $crate::core::logger::log_with_level($crate::core::logger::LogLevel::Alert, module_path!(), &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_crit {
    ($($arg:tt)*) => {
        $crate::core::logger::log_with_level($crate::core::logger::LogLevel::Critical, module_path!(), &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::core::logger::log_with_level($crate::core::logger::LogLevel::Error, module_path!(), &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::core::logger::log_with_level($crate::core::logger::LogLevel::Warning, module_path!(), &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_notice {
    ($($arg:tt)*) => {
        $crate::core::logger::log_with_level($crate::core::logger::LogLevel::Notice, module_path!(), &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::core::logger::log_with_level($crate::core::logger::LogLevel::Info, module_path!(), &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        $crate::core::logger::log_with_level($crate::core::logger::LogLevel::Debug, module_path!(), &format!($($arg)*))
    };
}

/// Internal function to log with level
pub fn log_with_level(level: LogLevel, target: &str, message: &str) {
    if let Ok(logger_guard) = LOGGER.lock() {
        if let Some(ref logger) = *logger_guard {
            logger.log(level, target, message);
        }
    }
}

/// Systemd-style initialization message with loading indicator
pub fn log_init_start(service_name: &str) {
    let message = format!("[*] Starting {}...", service_name);
    log_with_level(LogLevel::Notice, module_path!(), &message);
}

/// Systemd-style success message with OK status
#[allow(dead_code)] // Convenience functions for common logging patterns
pub fn log_init_ok(service_name: &str) {
    let message = format!("[   OK   ] Starting {}", service_name);
    log_with_level(LogLevel::Info, module_path!(), &message);
}

/// Systemd-style success message with details
#[allow(dead_code)] // Convenience functions for common logging patterns
pub fn log_init_ok_with_details(service_name: &str, details: &str) {
    let message = format!("[   OK   ] Starting {} ({})", service_name, details);
    log_with_level(LogLevel::Info, module_path!(), &message);
}

/// Systemd-style failure message with FAILED status
pub fn log_init_failed(service_name: &str, error: &str) {
    let message = format!("[  FAILED ] Starting {} - {}", service_name, error);
    log_with_level(LogLevel::Error, module_path!(), &message);
}

/// Systemd-style warning message with WARN status
pub fn log_init_warn(service_name: &str, warning: &str) {
    let message = format!("[   WARN ] Starting {} - {}", service_name, warning);
    log_with_level(LogLevel::Warning, module_path!(), &message);
}

/// Systemd-style service status message
#[allow(dead_code)] // Convenience functions for common logging patterns
pub fn log_service_status(service_name: &str, status: &str) {
    let message = format!("[*] {}: {}", service_name, status);
    log_with_level(LogLevel::Notice, module_path!(), &message);
}

/// Systemd-style task starting message
pub fn log_task_start(task_name: &str) {
    let message = format!("[*] {}... ", task_name);
    log_with_level(LogLevel::Notice, module_path!(), &message);
}

/// Systemd-style task completion message
#[allow(dead_code)] // Convenience functions for common logging patterns
pub fn log_task_complete(task_name: &str) {
    let message = format!("[   OK   ] {}", task_name);
    log_with_level(LogLevel::Info, module_path!(), &message);
}

/// Systemd-style task completion with details
#[allow(dead_code)] // Convenience functions for common logging patterns
pub fn log_task_complete_with_details(task_name: &str, details: &str) {
    let message = format!("[   OK   ] {} ({})", task_name, details);
    log_with_level(LogLevel::Info, module_path!(), &message);
}

/// Show progress bar for systemd-style loading
#[allow(dead_code)] // Convenience functions for common logging patterns
pub fn log_progress_start(service_name: &str, total_steps: usize) {
    let message = format!("[*] Starting {} ({} steps)...", service_name, total_steps);
    log_with_level(LogLevel::Notice, module_path!(), &message);
}

#[allow(dead_code)] // Convenience functions for common logging patterns
pub fn log_progress_step(service_name: &str, current_step: usize, total_steps: usize, step_name: &str) {
    let percentage = (current_step * 100) / total_steps;
    let bar_length = 20;
    let filled_length = (current_step * bar_length) / total_steps;

    let mut bar = String::new();
    for i in 0..bar_length {
        if i < filled_length {
            bar.push('=');
        } else if i == filled_length {
            bar.push('>');
        } else {
            bar.push(' ');
        }
    }

    let message = format!("[*] {} [{:3}%] [{}] {}", service_name, percentage, bar, step_name);
    log_with_level(LogLevel::Notice, module_path!(), &message);
}

#[allow(dead_code)] // Convenience functions for common logging patterns
pub fn log_progress_complete(service_name: &str) {
    let message = format!("[   OK   ] {} [100%] [====================]", service_name);
    log_with_level(LogLevel::Info, module_path!(), &message);
}

/// Convenience macros for systemd-style logging
#[macro_export]
macro_rules! log_init_start {
    ($service:expr) => {
        $crate::core::logger::log_init_start($service);
    };
}

#[macro_export]
macro_rules! log_init_ok {
    ($service:expr) => {
        $crate::core::logger::log_init_ok($service);
    };
}

#[macro_export]
macro_rules! log_init_ok_with_details {
    ($service:expr, $details:expr) => {
        $crate::core::logger::log_init_ok_with_details($service, $details);
    };
}

#[macro_export]
macro_rules! log_init_failed {
    ($service:expr, $error:expr) => {
        $crate::core::logger::log_init_failed($service, $error);
    };
}

#[macro_export]
macro_rules! log_init_warn {
    ($service:expr, $warning:expr) => {
        $crate::core::logger::log_init_warn($service, $warning);
    };
}

#[macro_export]
macro_rules! log_service_status {
    ($service:expr, $status:expr) => {
        $crate::core::logger::log_service_status($service, $status);
    };
}

#[macro_export]
macro_rules! log_task_start {
    ($task:expr) => {
        $crate::core::logger::log_task_start($task);
    };
}

#[macro_export]
macro_rules! log_task_complete {
    ($task:expr) => {
        $crate::core::logger::log_task_complete($task);
    };
}

#[macro_export]
macro_rules! log_task_complete_with_details {
    ($task:expr, $details:expr) => {
        $crate::core::logger::log_task_complete_with_details($task, $details);
    };
}

#[macro_export]
macro_rules! log_progress_start {
    ($service:expr, $steps:expr) => {
        $crate::core::logger::log_progress_start($service, $steps);
    };
}

#[macro_export]
macro_rules! log_progress_step {
    ($service:expr, $current:expr, $total:expr, $step:expr) => {
        $crate::core::logger::log_progress_step($service, $current, $total, $step);
    };
}

#[macro_export]
macro_rules! log_progress_complete {
    ($service:expr) => {
        $crate::core::logger::log_progress_complete($service);
    };
}

/// Logger initialization errors
#[derive(Debug, thiserror::Error)]
pub enum LoggerError {
    #[error("Logger already initialized")]
    AlreadyInitialized,
    #[error("Failed to initialize logger")]
    InitError,
}

/// Initialize logger from CLI arguments
pub fn init_from_args(debug: bool, trace: bool, journald: bool) -> Result<(), LoggerError> {
    let min_level = if trace {
        LogLevel::Debug
    } else if debug {
        LogLevel::Debug
    } else {
        LogLevel::Info
    };

    let config = LoggerConfig {
        min_level,
        use_colors: atty::is(atty::Stream::Stderr) && !journald,
        include_timestamp: !journald,
        include_target: trace,
        journald_format: journald,
    };

    Logger::init(config)
}

/// Get the current minimum log level
#[allow(dead_code)] // Global convenience functions
pub fn get_min_level() -> LogLevel {
    if let Ok(logger_guard) = LOGGER.lock() {
        if let Some(ref logger) = *logger_guard {
            return LogLevel::from_priority(logger.min_level.load(Ordering::Relaxed));
        }
    }
    LogLevel::Info
}

/// Check if we should log at the given level
#[allow(dead_code)] // Global convenience functions
pub fn should_log(level: LogLevel) -> bool {
    if let Ok(logger_guard) = LOGGER.lock() {
        if let Some(ref logger) = *logger_guard {
            return logger.should_log(level);
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Emergency < LogLevel::Alert);
        assert!(LogLevel::Info < LogLevel::Debug);
        assert!(LogLevel::Error > LogLevel::Warning);
    }

    #[test]
    fn test_log_level_priority() {
        assert_eq!(LogLevel::Emergency.priority(), 0);
        assert_eq!(LogLevel::Info.priority(), 6);
        assert_eq!(LogLevel::Debug.priority(), 7);
    }

    #[test]
    fn test_logger_config_default() {
        let config = LoggerConfig::default();
        assert_eq!(config.min_level, LogLevel::Info);
        assert!(config.include_timestamp);
        assert!(!config.journald_format);
    }

    #[test]
    fn test_logger_level_filtering() {
        let config = LoggerConfig {
            min_level: LogLevel::Warning,
            ..Default::default()
        };
        let logger = Logger::new(config);

        assert!(logger.should_log(LogLevel::Error));
        assert!(logger.should_log(LogLevel::Warning));
        assert!(!logger.should_log(LogLevel::Info));
        assert!(!logger.should_log(LogLevel::Debug));
    }
}