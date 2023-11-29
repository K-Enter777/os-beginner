use uefi::proto::console::gop::ModeInfo;

pub struct GraphicsInfo {
    pub pixel_info: ModeInfo,
    pub frame_buffer_base: usize,
    pub frame_buffer_size: usize,
}
