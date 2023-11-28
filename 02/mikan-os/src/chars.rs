use core::fmt::Write;

use uefi::{cstr16, CStr16};

pub struct Str16Buf<'a> {
    buf: &'a mut [u16],
    pos: usize,
}

impl<'a> Str16Buf<'a> {
    pub fn new(buf: &'a mut [u16]) -> Self {
        Self { buf, pos: 0 }
    }

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

pub struct Str8Buf<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> Str8Buf<'a> {
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }

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
