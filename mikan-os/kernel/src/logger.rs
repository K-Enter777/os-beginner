#![allow(unused)]

use core::ffi::{c_char, CStr};

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[repr(C)]
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
            $crate::printkln!("{}", format_args!($($arg)*));
        }
    }
}

// #[export_name = "_Z3LogLogLevelPKcz"]
// pub(crate) fn log_cpp(level: LogLevel, format: *const c_char) -> i32 {
//     let s = unsafe { CStr::from_ptr(format) }
//         .to_str()
//         .expect("Can't transform.");

//     use crate::{printk, printkln, CONSOLE};
//     use core::fmt::Write;
//     log!(level, "{}", s);

//     return 0;
// }
