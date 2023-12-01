#![no_std]
#![no_main]

mod graphics;
mod placement;

use core::{arch::asm, mem::size_of, panic::PanicInfo};
use graphics::{
    BgrResv8BitPerColorPixelWriter, FrameBufferConfig, PixelColor, PixelFormat, PixelWrite,
    RgbResv8BitPerColorPixelWriter,
};
use placement::new_mut_with_buf;

#[no_mangle]
pub extern "sysv64" fn kernel_entry(frame_buffer_config: FrameBufferConfig) {
    let pixel_writer_buf = [0u8; size_of::<RgbResv8BitPerColorPixelWriter>()];
    let pixel_writer: &mut dyn PixelWrite = match frame_buffer_config.pixel_format {
        PixelFormat::Rgb => {
            match new_mut_with_buf(
                RgbResv8BitPerColorPixelWriter::new(frame_buffer_config),
                &pixel_writer_buf,
            ) {
                Err(_size) => halt(),
                Ok(writer) => writer,
            }
        }
        PixelFormat::Bgr => {
            match new_mut_with_buf(
                BgrResv8BitPerColorPixelWriter::new(frame_buffer_config),
                &pixel_writer_buf,
            ) {
                Err(_size) => halt(),
                Ok(writer) => writer,
            }
        }
    };

    for x in 0..pixel_writer.config().horizontal_resolution {
        for y in 0..pixel_writer.config().vertical_resolution {
            pixel_writer.write(x, y, &PixelColor::new(u8::MAX, u8::MAX, u8::MAX));
        }
    }

    for x in 0..200 {
        for y in 0..100 {
            pixel_writer.write(100 + x, 100 + y, &PixelColor::new(0, 255, 0));
        }
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
