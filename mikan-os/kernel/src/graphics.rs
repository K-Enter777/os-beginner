#![allow(unused)]

use core::slice;

use crate::k_font_a;

#[repr(C)]
#[derive(Clone, Copy)]
pub enum PixelFormat {
    Rgb,
    Bgr,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FrameBufferConfig {
    pub frame_buffer: usize,
    pub pixels_per_scan_line: usize,
    pub horizontal_resolution: usize,
    pub vertical_resolution: usize,
    pub pixel_format: PixelFormat,
}

pub struct PixelColor {
    r: u8,
    g: u8,
    b: u8,
}

impl PixelColor {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

pub(crate) trait PixelWriter {
    fn write(&self, x: usize, y: usize, color: &PixelColor);
    fn config(&self) -> &FrameBufferConfig;

    fn write_ascii(&self, x: usize, y: usize, c: u8, color: &PixelColor) {
        if c != b'A' {
            return;
        }
        for dy in 0..16 {
            for dx in 0..8 {
                if ((k_font_a[dy] << dx) & 0x80) != 0 {
                    self.write(x + dx, y + dy, color);
                }
            }
        }
    }

    #[inline]
    fn pixel(&self, x: usize, y: usize) -> &mut [u8] {
        unsafe {
            slice::from_raw_parts_mut(
                (self.config().frame_buffer + 4 * (self.config().pixels_per_scan_line * y + x))
                    as *mut u8,
                3,
            )
        }
    }
}

pub(crate) struct RgbResv8BitPerColorPixelWriter {
    config: FrameBufferConfig,
}

impl RgbResv8BitPerColorPixelWriter {
    pub(crate) fn new(config: FrameBufferConfig) -> Self {
        Self { config }
    }
}

impl PixelWriter for RgbResv8BitPerColorPixelWriter {
    fn write(&self, x: usize, y: usize, color: &PixelColor) {
        let pixel = self.pixel(x, y);
        pixel[0] = color.r;
        pixel[1] = color.g;
        pixel[2] = color.b;
    }

    fn config(&self) -> &FrameBufferConfig {
        &self.config
    }
}

pub(crate) struct BgrResv8BitPerColorPixelWriter {
    config: FrameBufferConfig,
}

impl BgrResv8BitPerColorPixelWriter {
    pub(crate) fn new(config: FrameBufferConfig) -> Self {
        Self { config }
    }
}

impl PixelWriter for BgrResv8BitPerColorPixelWriter {
    fn write(&self, x: usize, y: usize, color: &PixelColor) {
        let pixel = self.pixel(x, y);
        pixel[0] = color.b;
        pixel[1] = color.g;
        pixel[2] = color.r;
    }

    fn config(&self) -> &FrameBufferConfig {
        &self.config
    }
}
