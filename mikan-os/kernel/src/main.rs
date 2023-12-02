#![no_std]
#![no_main]

mod graphics;
mod placement;

use core::{arch::asm, mem::size_of, panic::PanicInfo};
use graphics::{
    BgrResv8BitPerColorPixelWriter, FrameBufferConfig, PixelColor, PixelFormat, PixelWriter,
    RgbResv8BitPerColorPixelWriter,
};
use placement::new_mut_with_buf;

const k_font_a: [u8; 16] = [
    0b00000000, //
    0b00011000, //    **
    0b00011000, //    **
    0b00011000, //    **
    0b00011000, //    **
    0b00100100, //   *  *
    0b00100100, //   *  *
    0b00100100, //   *  *
    0b00100100, //   *  *
    0b01111110, //  ******
    0b01000010, //  *    *
    0b01000010, //  *    *
    0b01000010, //  *    *
    0b11100111, // ***  ***
    0b00000000, //
    0b00000000, //
];

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

    // 緑の長方形の描画
    for x in 0..200 {
        for y in 0..100 {
            pixel_writer.write(x, y, &PixelColor::new(0, 255, 0));
        }
    }

    // 文字の描画
    pixel_writer.write_ascii(50, 50, b'A', &PixelColor::new(0, 0, 0));
    pixel_writer.write_ascii(58, 50, b'A', &PixelColor::new(0, 0, 0));

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
