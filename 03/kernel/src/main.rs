#![no_std]
#![no_main]
use core::{arch::asm, ffi, panic::PanicInfo, slice};

#[no_mangle]
pub extern "sysv64" fn kernel_entry(
    frame_buffer_base: ffi::c_ulong,
    frame_buffer_size: ffi::c_ulong,
) {
    let frame_buffer = unsafe {
        slice::from_raw_parts_mut(frame_buffer_base as *mut u8, frame_buffer_size as usize)
    };

    for i in 0..frame_buffer.len() {
        frame_buffer[i] = (i % u8::MAX as usize) as u8;
    }

    // なぜかこれでは動かなくなる
    // RIP の値がおかしくなっているので、イテレーターを作るときに変なとこにアクセスしている？
    // for pixel in frame_buffer.iter_mut() {
    //     *pixel = u8::MAX;
    // }

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
