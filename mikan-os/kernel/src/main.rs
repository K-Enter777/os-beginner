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
};
use placement::new_mut_with_buf;
use string::StringU8;

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
            pixel_writer.write(x, y, &PixelColor::new(u8::MAX, u8::MAX, u8::MAX));
        }
    }

    let mut console = Console::new(
        pixel_writer,
        PixelColor::new(0, 0, 0),
        PixelColor::new(255, 255, 255),
    );

    let mut buf = [0u8; 128];
    let mut str_buf = StringU8::new(&mut buf);
    // write!(str_buf, "line {}\n", 0).unwrap();
    // console.put_string(str_buf.to_string());
    for i in 0..27 {
        str_buf.clear();
        write!(str_buf, "line {}\n", i).unwrap();
        console.put_string(str_buf.to_string());
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
