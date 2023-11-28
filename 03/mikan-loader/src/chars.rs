use core::fmt::Write;

use uefi::{cstr16, CStr16};

/// ヒープが使えない環境で、16 bit 文字列のフォーマットを行うための構造体。
pub struct Str16Buf<'a> {
    buf: &'a mut [u16],
    pos: usize,
}

impl<'a> Str16Buf<'a> {
    /// フォーマットを行うためのバッファから、構造体を作る。
    pub fn new(buf: &'a mut [u16]) -> Self {
        Self { buf, pos: 0 }
    }

    /// 所持しているバッファから UEFI 用の 16 bit 文字列を返す。
    pub fn into_cstr16(&self) -> &CStr16 {
        match CStr16::from_u16_with_nul(&self.buf[..=self.pos]) {
            Ok(s) => s,
            Err(_) => cstr16!(),
        }
    }
}

impl<'a> Write for Str16Buf<'a> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let mut s = s.chars();
        while let Some(item) = s.next() {
            if self.pos == self.buf.len() - 1 {
                break;
            }
            self.buf[self.pos] = item as u16;
            self.pos += 1;
        }
        self.buf[self.pos] = 0;

        Ok(())
    }
}

/// ヒープが使えない環境で、8 bit 文字列のフォーマットを行うための構造体。
pub struct Str8Buf<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> Str8Buf<'a> {
    /// フォーマットを行うためのバッファから、構造体を作る。
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    /// 8 bit 文字列のバッファを返す。
    pub fn get(&self) -> &[u8] {
        &self.buf[..self.pos]
    }
}

impl<'a> Write for Str8Buf<'a> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let mut s = s.chars();
        while let Some(item) = s.next() {
            if self.pos == self.buf.len() - 1 {
                break;
            }
            self.buf[self.pos] = item as u8;
            self.pos += 1;
        }

        Ok(())
    }
}
