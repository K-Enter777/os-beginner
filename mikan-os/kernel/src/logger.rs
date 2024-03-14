#![allow(unused)]

use core::{
    ffi::{c_char, CStr},
    mem::size_of,
    panic,
};

use alloc::{format, string::String};

use crate::{printk, sync::RwLock};

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
    *LOG_LEVEL.write() = level;
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

/// C++ 側から呼び出されるログ出力関数。
///
/// # Note
/// 対応しているフォーマット指定子は以下の通り。
///
/// - `%s`: 文字列
/// - `%d`: 10進整数
/// - `%x`: 16進整数
/// - `%u`: 符号なし10進整数
/// - `%c`: 文字
/// - `%p`: ポインタ
///
/// 必要に応じて今後増加させる。
///
/// また、本来は `AL` レジスタが引数に使用されているベクターレジスタの数を保存しているが、
/// `AL` レジスタを取得するのは大変なため、浮動小数点数は非対応。
#[export_name = "_Z3Log8LogLevelPKcz"]
pub(crate) extern "sysv64" fn log_cpp(
    level: LogLevel,
    format: *const c_char,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    args: u64,
) -> i32 {
    // ログレベル以下のログは出力しない
    if level > get_log_level() {
        return 0;
    }

    let s = unsafe { CStr::from_ptr(format) }
        .to_str()
        .expect("Can't transform.");

    // スタックに積まれた引数のポインタ
    let p_args = &args as *const u64;

    // 既に読んだ引数の数
    let mut num_read = 0;

    // `%` のあとの文字によっては引数を変換する必要があるため、
    // `%` のあとであるかを管理する
    let mut arg_maybe_needed = false;

    // 最終的に表示する文字列
    let mut str = String::with_capacity(s.len());
    for c in s.chars() {
        if arg_maybe_needed {
            // 引数を u64 として取得
            let arg = match num_read {
                0 => arg1,
                1 => arg2,
                2 => arg3,
                3 => arg4,
                // 7 つ目以降の引数はスタックに積まれているため、ポインタを進めて取得
                // pointer.add() は最適化されて（？）使えないため、pointer.byte_add() を使用
                i => unsafe { *p_args.byte_add((i - 4) * size_of::<u64>()) },
            };

            match c {
                // 文字列
                's' => {
                    let s = unsafe { CStr::from_ptr(arg as *const c_char) }
                        .to_str()
                        .unwrap();
                    str.push_str(s);
                }
                // 10進整数
                'd' => str.push_str(&format!("{}", arg as i64)),
                // 16進整数
                'x' => str.push_str(&format!("{:x}", arg)),
                // 符号なし10進整数
                'u' => str.push_str(&format!("{}", arg)),
                // 文字
                'c' => str.push(arg as u8 as char),
                // ポインタ
                'p' => str.push_str(&format!("{:p}", arg as *const u8)),
                // % のエスケープ
                '%' => {
                    arg_maybe_needed = false;
                    str.push('%');
                    str.push(c);
                    continue;
                }
                // 非対応
                _ => panic!("Unknown format specifier: %{}", c),
            }
            arg_maybe_needed = false;
            num_read += 1;
        } else {
            if c == '%' {
                arg_maybe_needed = true;
            } else {
                str.push(c);
            }
        }
    }

    // 元々の関数が改行コードを勝手に付けないため、こちらもそのようにする
    printk!("{}", str);

    0
}
