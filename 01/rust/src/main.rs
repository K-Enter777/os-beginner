#![no_std]
#![no_main]

use uefi::prelude::*;

#[entry]
fn efi_main(_image_handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut system_table).unwrap();

    system_table
        .stdout()
        .output_string(cstr16!("Hello, World!\r\n"))
        .unwrap();
    loop {}
}
