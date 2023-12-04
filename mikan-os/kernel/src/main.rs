#![no_std]
#![no_main]

mod console;
mod font;
mod font_data;
mod frame_buffer_config;
mod graphics;
mod placement;
mod string;

use console::Console;
use core::{arch::asm, fmt::Write, mem::size_of, panic::PanicInfo};
use frame_buffer_config::{FrameBufferConfig, PixelFormat};
use graphics::{
    BgrResv8BitPerColorPixelWriter, PixelColor, PixelWriter, RgbResv8BitPerColorPixelWriter,
    Vector2D,
};
use placement::new_mut_with_buf;

#[no_mangle]
pub extern "sysv64" fn kernel_entry(frame_buffer_config: FrameBufferConfig) {
    let mut pixel_writer_buf = [0u8; size_of::<RgbResv8BitPerColorPixelWriter>()];
    let pixel_writer: &mut dyn PixelWriter = match frame_buffer_config.pixel_format {
        PixelFormat::Rgb => {
            match new_mut_with_buf(
                RgbResv8BitPerColorPixelWriter::new(frame_buffer_config),
                &mut pixel_writer_buf,
            ) {
                Err(_size) => halt(),
                Ok(writer) => writer,
            }
        }
        PixelFormat::Bgr => {
            match new_mut_with_buf(
                BgrResv8BitPerColorPixelWriter::new(frame_buffer_config),
                &mut pixel_writer_buf,
            ) {
                Err(_size) => halt(),
                Ok(writer) => writer,
            }
        }
    };

    // 背景を白で塗りつぶす
    for x in 0..pixel_writer.config().horizontal_resolution {
        for y in 0..pixel_writer.config().vertical_resolution {
            pixel_writer.write(
                Vector2D::new(x as u32, y as u32),
                &PixelColor::new(255, 255, 255),
            );
        }
    }

    // コンソールの生成
    let mut console = Console::new(
        pixel_writer,
        PixelColor::new(0, 0, 0),
        PixelColor::new(255, 255, 255),
    );

    // line i を 0 <= i < 27 でコンソールに出力
    for i in 0..27 {
        write!(console, "line {}\n", i).unwrap();
    }

    halt();
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    halt()
}

fn halt() -> ! {
    loop {
        unsafe {
            asm!("hlt");
        }
    }
}
