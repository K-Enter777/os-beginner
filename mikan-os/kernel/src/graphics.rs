#![allow(unused)]

use core::slice;

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

pub(crate) trait PixelWrite {
    fn write(&self, x: usize, y: usize, color: &PixelColor);
    fn config(&self) -> &FrameBufferConfig;

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

impl PixelWrite for RgbResv8BitPerColorPixelWriter {
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

impl PixelWrite for BgrResv8BitPerColorPixelWriter {
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
