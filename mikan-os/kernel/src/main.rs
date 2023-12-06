#![no_std]
#![no_main]

mod console;
mod error;
mod font;
mod font_data;
mod frame_buffer_config;
mod graphics;
mod io;
mod pci;
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

/// デスクトップ背景の色
const DESKTOP_BG_COLOR: PixelColor = PixelColor::new(45, 118, 237);
/// デスクトップ前景の色
const DESKTOP_FG_COLOR: PixelColor = PixelColor::new(255, 255, 255);

/// マウスカーソルの横幅
const MOUSE_CURSOR_WIDTH: usize = 15;
/// マウスカーソルの高さ
const MOUSE_CURSOR_HEIGHT: usize = 24;
/// マウスカーソルの形
const MOUSE_CURSOR_SHAPE: [&[u8; MOUSE_CURSOR_WIDTH]; MOUSE_CURSOR_HEIGHT] = [
    b"@              ",
    b"@@             ",
    b"@.@            ",
    b"@..@           ",
    b"@...@          ",
    b"@....@         ",
    b"@.....@        ",
    b"@......@       ",
    b"@.......@      ",
    b"@........@     ",
    b"@.........@    ",
    b"@..........@   ",
    b"@...........@  ",
    b"@............@ ",
    b"@......@@@@@@@@",
    b"@......@       ",
    b"@....@@.@      ",
    b"@...@ @.@      ",
    b"@..@   @.@     ",
    b"@.@    @.@     ",
    b"@@      @.@    ",
    b"@       @.@    ",
    b"         @.@   ",
    b"         @@@   ",
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

    let frame_width = pixel_writer.config().horizontal_resolution as u32;
    let frame_height = pixel_writer.config().vertical_resolution as u32;

    // デスクトップ背景の描画
    pixel_writer.fill_rectangle(
        Vector2D::new(0, 0),
        Vector2D::new(frame_width, frame_height - 50),
        &DESKTOP_BG_COLOR,
    );
    // タスクバーの表示
    pixel_writer.fill_rectangle(
        Vector2D::new(0, frame_height - 50),
        Vector2D::new(frame_width, 50),
        &PixelColor::new(1, 8, 17),
    );
    // （多分）Windows の検索窓
    pixel_writer.fill_rectangle(
        Vector2D::new(0, frame_height - 50),
        Vector2D::new(frame_width / 5, 50),
        &PixelColor::new(80, 80, 80),
    );
    // （多分）Windows のスタートボタン
    pixel_writer.fill_rectangle(
        Vector2D::new(10, frame_height - 40),
        Vector2D::new(30, 30),
        &PixelColor::new(160, 160, 160),
    );

    // コンソールの生成
    let mut console = Console::new(pixel_writer, &DESKTOP_FG_COLOR, &DESKTOP_BG_COLOR);

    // welcome 文
    console.put_string(b"Welcome to MikanOS!\n");

    // マウスカーソルの描画
    for dy in 0..MOUSE_CURSOR_HEIGHT {
        for dx in 0..MOUSE_CURSOR_WIDTH {
            if MOUSE_CURSOR_SHAPE[dy][dx] == b'@' {
                pixel_writer.write(
                    Vector2D::new(200 + dx as u32, 100 + dy as u32),
                    &PixelColor::new(0, 0, 0),
                );
            } else if MOUSE_CURSOR_SHAPE[dy][dx] == b'.' {
                pixel_writer.write(
                    Vector2D::new(200 + dx as u32, 100 + dy as u32),
                    &PixelColor::new(255, 255, 255),
                );
            }
        }
    }

    // デバイス一覧の表示
    let err = pci::scan_all_bus();
    write!(console, "scan_all_bus: {}\n", err).unwrap();

    let devices = pci::DEVICES.lock().take();
    let num_devices = pci::NUM_DEVICES.lock().take();
    for i in 0..num_devices {
        let dev = devices[i].unwrap();
        let vendor_id = pci::read_vendor_id(dev.bus(), dev.device(), dev.function());
        let class_code = pci::read_class_code(dev.bus(), dev.device(), dev.function());
        write!(
            console,
            "{}.{}.{}: vend {:04x}, class {:08x}, head {:02x}\n",
            dev.bus(),
            dev.device(),
            dev.function(),
            vendor_id,
            class_code,
            dev.header_type()
        )
        .unwrap();
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
