use chrono::Utc;

pub fn manual(level: &str, message: &str) {
    let timestamp = Utc::now().format(
        "%Y-%m-%dT%H:%M:%S%.3fZ"
    );
    println!("{} {} {}", timestamp, level, message);
}

macro_rules! log_trace {
    ($($arg:tt)*) => {
        log::manual("TRACE", &format!($($arg)*));
    };
}

macro_rules! log_debug {
    ($($arg:tt)*) => {
        log::manual("DEBUG", &format!($($arg)*));
    };
}

macro_rules! log_info {
    ($($arg:tt)*) => {
        log::manual("INFO", &format!($($arg)*));
    };
}

macro_rules! log_warn {
    ($($arg:tt)*) => {
        log::manual("WARN", &format!($($arg)*));
    };
}

macro_rules! log_error {
    ($($arg:tt)*) => {
        log::manual("ERROR", &format!($($arg)*));
    };
}

macro_rules! log_critical {
    ($($arg:tt)*) => {
        log::manual("CRITICAL", &format!($($arg)*));
    };
}
