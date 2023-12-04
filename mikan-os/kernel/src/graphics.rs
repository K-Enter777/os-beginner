#![allow(unused)]

use core::{
    ops::{Add, AddAssign, Sub, SubAssign},
    slice,
};

use crate::frame_buffer_config::FrameBufferConfig;

#[derive(PartialEq, Eq)]
pub struct PixelColor {
    r: u8,
    g: u8,
    b: u8,
}

/// ピクセルの色情報を持つ。
impl PixelColor {
    /// 初期化。
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// 32 bit 情報から [PixelColor] へ変換する。
    pub(crate) fn to_color(c: u32) -> Self {
        Self {
            r: (c >> 16) as u8 & 0xff,
            g: (c >> 8) as u8 & 0xff,
            b: c as u8 & 0xff,
        }
    }
}

/// ピクセルを塗るための色々を提供する。
pub(crate) trait PixelWriter {
    /// ピクセルを塗る手段を提供する。
    fn write(&self, pos: Vector2D<u32>, color: &PixelColor);
    /// フレームバッファの情報を提供する。
    fn config(&self) -> &FrameBufferConfig;

    #[inline]
    /// ピクセルの位置から、そのピクセルを塗るための配列を提供する。
    fn pixel_at(&self, pos: Vector2D<u32>) -> &mut [u8] {
        unsafe {
            slice::from_raw_parts_mut(
                (self.config().frame_buffer
                    + 4 * (self.config().pixels_per_scan_line * pos.y as usize + pos.x as usize))
                    as *mut u8,
                3,
            )
        }
    }

    /// 長方形の枠を指定された色で塗る。
    fn draw_rectangle(&mut self, pos: Vector2D<u32>, size: Vector2D<u32>, c: &PixelColor) {
        // 横線
        for dx in 0..size.x {
            self.write(pos + Vector2D::new(dx, 0), c);
            self.write(pos + Vector2D::new(dx, size.y - 1), c);
        }

        // 縦線
        for dy in 0..size.y {
            self.write(pos + Vector2D::new(0, dy), c);
            self.write(pos + Vector2D::new(size.x - 1, dy), c);
        }
    }

    // 長方形を指定された色で塗る。
    fn fill_rectangle(&mut self, pos: Vector2D<u32>, size: Vector2D<u32>, c: &PixelColor) {
        for dy in 0..size.y {
            for dx in 0..size.x {
                self.write(pos + Vector2D::new(dx, dy), c);
            }
        }
    }
}

/// フレームバッファのピクセルの持ち方が RGB のときのクラス。
pub(crate) struct RgbResv8BitPerColorPixelWriter {
    config: FrameBufferConfig,
}

impl RgbResv8BitPerColorPixelWriter {
    pub(crate) fn new(config: FrameBufferConfig) -> Self {
        Self { config }
    }
}

impl PixelWriter for RgbResv8BitPerColorPixelWriter {
    fn write(&self, pos: Vector2D<u32>, color: &PixelColor) {
        let pixel = self.pixel_at(pos);
        pixel[0] = color.r;
        pixel[1] = color.g;
        pixel[2] = color.b;
    }

    fn config(&self) -> &FrameBufferConfig {
        &self.config
    }
}

/// フレームバッファのピクセルの持ち方が BGR のときのクラス。
pub(crate) struct BgrResv8BitPerColorPixelWriter {
    config: FrameBufferConfig,
}

impl BgrResv8BitPerColorPixelWriter {
    /// 初期化。
    pub(crate) fn new(config: FrameBufferConfig) -> Self {
        Self { config }
    }
}

impl PixelWriter for BgrResv8BitPerColorPixelWriter {
    fn write(&self, pos: Vector2D<u32>, color: &PixelColor) {
        let pixel = self.pixel_at(pos);
        pixel[0] = color.b;
        pixel[1] = color.g;
        pixel[2] = color.r;
    }

    fn config(&self) -> &FrameBufferConfig {
        &self.config
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
/// 2次元のベクトル情報を保持するクラス。
pub(crate) struct Vector2D<T> {
    x: T,
    y: T,
}

impl<T: Copy + Add + AddAssign + Sub + SubAssign> Vector2D<T> {
    /// 初期化。
    #[inline]
    pub(crate) const fn new(x: T, y: T) -> Self {
        Self { x, y }
    }

    /// x 成分を返す。
    #[inline]
    pub(crate) const fn x(&self) -> T {
        self.x
    }

    /// y 成分を返す。
    #[inline]
    pub(crate) const fn y(&self) -> T {
        self.y
    }
}

/// 成分の加算を加算として定義する。
impl<T> Add for Vector2D<T>
where
    T: Add<Output = T>,
{
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

/// 成分の加算を加算として定義する。
impl<T: AddAssign> AddAssign for Vector2D<T> {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

/// 成分の減算を減算として定義する。
impl<T> Sub for Vector2D<T>
where
    T: Sub<Output = T>,
{
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

/// 成分の減算を減算として定義する。
impl<T: SubAssign> SubAssign for Vector2D<T> {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}
