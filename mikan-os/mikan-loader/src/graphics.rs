use uefi::proto::console::gop::ModeInfo;

pub struct GraphicsInfo {
    pub pixel_info: ModeInfo,
    pub frame_buffer_base: usize,
    pub frame_buffer_size: usize,
}

#[repr(C)]
pub enum PixelFormat {
    Rgb,
    Bgr,
}

#[repr(C)]
pub struct FrameBufferConfig {
    pub frame_buffer: usize,
    pub pixels_per_scan_line: usize,
    pub horizontal_resolution: usize,
    pub vertical_resolution: usize,
    pub pixel_format: PixelFormat,
}
