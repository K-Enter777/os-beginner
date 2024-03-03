#![allow(unused)]

use core::ffi::{c_char, CStr};

use crate::sync::RwLock;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[repr(C)]
pub(crate) enum LogLevel {
    Error = 3,
    Warn = 4,
    Info = 6,
    Debug = 7,
}

static LOG_LEVEL: RwLock<LogLevel> = RwLock::new(LogLevel::Warn);

pub(crate) fn set_log_level(level: LogLevel) {
    unsafe {
        *LOG_LEVEL.write() = level;
    }
}

pub(crate) fn get_log_level() -> LogLevel {
    *LOG_LEVEL.read()
}

#[macro_export]
macro_rules! log {
    ($level:expr, $($arg:tt)*) => {
        if $level <= $crate::logger::get_log_level() {
            $crate::printkln!("{}", format_args!($($arg)*));
        }
    }
}

#[export_name = "_Z3Log8LogLevelPKcz"]
pub(crate) fn log_cpp(level: LogLevel, format: *const c_char) -> i32 {
    let s = unsafe { CStr::from_ptr(format) }
        .to_str()
        .expect("Can't transform.");

    use crate::{printk, printkln, CONSOLE};
    use core::fmt::Write;
    log!(level, "{}", s);

    return 0;
}
