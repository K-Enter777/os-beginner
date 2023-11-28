#![no_std]
#![no_main]

use core::panic::PanicInfo;
use r_efi::efi;
use utf16_literal::utf16;

#[export_name = "efi_main"]
pub extern "C" fn efi_main(
    _image_handle: efi::Handle,
    system_table: *mut efi::SystemTable,
) -> efi::Status {
    let buf = utf16!("Hello, world!\n\0");

    unsafe {
        ((*(*system_table).con_out).output_string)(
            (*system_table).con_out,
            buf.as_ptr() as *mut efi::Char16,
        )
    };
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
