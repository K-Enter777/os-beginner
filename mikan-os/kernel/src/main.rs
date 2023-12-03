#![no_std]
#![no_main]

mod font;
mod font_data;
mod frame_buffer_config;
mod graphics;
mod placement;
mod string;

use core::{arch::asm, fmt::Write, mem::size_of, panic::PanicInfo};
use font::{write_ascii, write_string};
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

    // 緑の長方形の描画
    for x in 0..200 {
        for y in 0..100 {
            pixel_writer.write(x, y, &PixelColor::new(0, 255, 0));
        }
    }

    // 文字一覧を描画
    let mut i = 0;
    for c in b'!'..=b'~' {
        write_ascii(pixel_writer, 8 * i, 50, c, &PixelColor::new(0, 0, 0));
        i += 1;
    }

    // ハローワールド
    write_string(
        pixel_writer,
        0,
        66,
        b"Hello, world!",
        &PixelColor::new(0, 0, 255),
    );

    // ~~sprintf ~~ write! チェック
    let mut buf = [0u8; 128];
    let mut str_buf = StringU8::new(&mut buf);
    if let Err(_) = write!(str_buf, "計算: 1 + 2 = {}", 1 + 2) {
        write_string(
            pixel_writer,
            0,
            82,
            b"Characters must be ASCII!",
            &PixelColor::new(255, 0, 0),
        )
    }
    write_string(
        pixel_writer,
        0,
        82,
        str_buf.to_string(),
        &PixelColor::new(0, 0, 0),
    );
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
