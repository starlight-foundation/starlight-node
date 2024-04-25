use chrono::Utc;

pub fn manual(level: &str, message: &str) {
    let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ");
    println!("{} {} {}", timestamp, level, message);
}

#[macro_export]
macro_rules! log_trace {
    ($($arg:tt)*) => {
        crate::app::log::manual("TRACE", &format!($($arg)*));
    };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        crate::app::log::manual("DEBUG", &format!($($arg)*));
    };
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        crate::app::log::manual("INFO", &format!($($arg)*));
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        crate::app::log::manual("WARN", &format!($($arg)*));
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        crate::app::log::manual("ERROR", &format!($($arg)*));
    };
}

#[macro_export]
macro_rules! log_critical {
    ($($arg:tt)*) => {
        crate::app::log::manual("CRITICAL", &format!($($arg)*));
    };
}
