#![allow(unused)]

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub(crate) enum LogLevel {
    Error = 3,
    Warn = 4,
    Info = 6,
    Debug = 7,
}

static mut LOG_LEVEL: LogLevel = LogLevel::Warn;

pub(crate) fn set_log_level(level: LogLevel) {
    unsafe {
        LOG_LEVEL = level;
    }
}

pub(crate) fn get_log_level() -> LogLevel {
    unsafe { LOG_LEVEL }
}

#[macro_export]
macro_rules! log {
    ($level:expr, $($arg:tt)*) => {
        if $level <= $crate::logger::get_log_level() {
            printkln!("{}", format_args!($($arg)*));
        }
    }
}
