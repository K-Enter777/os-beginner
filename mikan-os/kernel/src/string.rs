#![allow(unused)]

use core::fmt::{self, Write};

/// バッファによる ASCII 文字列の保持を行う。
pub(crate) struct StringU8<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> StringU8<'a> {
    /// 与えられたバッファから ASCII 文字列を作成。
    pub(crate) fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    /// 現在保持している文字列の長さを返す。
    pub(crate) fn len(&self) -> usize {
        self.pos
    }

    /// 保持している文字列の削除。
    pub(crate) fn clear(&mut self) {
        self.pos = 0;
    }

    /// 保持している文字列を [u8] スライスとして返す。
    pub(crate) fn to_string(&self) -> &[u8] {
        &self.buf[..self.len()]
    }
}

impl<'a> Write for StringU8<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut s = s.chars();
        while let Some(c) = s.next() {
            if !c.is_ascii() {
                return Err(fmt::Error);
            }
            if self.pos == self.buf.len() {
                break;
            }
            self.buf[self.pos] = c as u8;
            self.pos += 1;
        }

        Ok(())
    }
}
