#![no_std]
#![no_main]

mod graphics;

use core::{arch::asm, panic::PanicInfo, slice};
use graphics::{FrameBufferConfig, PixelFormat};

struct PixelColor {
    red: u8,
    green: u8,
    blue: u8,
}

fn write_pixel(config: &FrameBufferConfig, x: usize, y: usize, color: &PixelColor) {
    let pixel = unsafe {
        slice::from_raw_parts_mut(
            (4 * (config.pixels_per_scan_line * y + x) + config.frame_buffer) as *mut u8,
            3,
        )
    };
    match config.pixel_format {
        PixelFormat::Rgb => {
            pixel[0] = color.red;
            pixel[1] = color.green;
            pixel[2] = color.blue;
        }
        PixelFormat::Bgr => {
            pixel[0] = color.blue;
            pixel[1] = color.green;
            pixel[2] = color.red;
        }
    }
}

#[no_mangle]
pub extern "sysv64" fn kernel_entry(frame_buffer_config: FrameBufferConfig) {
    for x in 0..frame_buffer_config.horizontal_resolution {
        for y in 0..frame_buffer_config.vertical_resolution {
            write_pixel(
                &frame_buffer_config,
                x,
                y,
                &PixelColor {
                    red: u8::MAX,
                    green: u8::MAX,
                    blue: u8::MAX,
                },
            );
        }
    }

    for x in 0..200 {
        for y in 0..100 {
            write_pixel(
                &frame_buffer_config,
                100 + x,
                100 + y,
                &PixelColor {
                    red: 0,
                    green: u8::MAX,
                    blue: 0,
                },
            );
        }
    }

    loop {
        unsafe {
            asm!("hlt");
        }
    }
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}
