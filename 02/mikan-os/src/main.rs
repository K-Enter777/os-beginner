#![no_std]
#![no_main]

use core::panic::PanicInfo;
use r_efi::efi;
use utf16_literal::utf16;

struct MemoryMap {
    buffer_size: usize,
    buffer: *mut efi::MemoryDescriptor,
    map_size: usize,
    map_key: usize,
    descriptor_size: usize,
    descriptor_version: u32,
}

fn get_memory_map(get_memory_map: efi::BootGetMemoryMap, map: &mut MemoryMap) -> efi::Status {
    if map.buffer.is_null() {
        return efi::Status::BUFFER_TOO_SMALL;
    }

    map.map_size = map.buffer_size;
    get_memory_map(
        map.map_size as *mut usize,
        map.buffer,
        map.map_key as *mut usize,
        map.descriptor_size as *mut usize,
        map.descriptor_version as *mut u32,
    )
}

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

    let mut memmap_buf = [efi::MemoryDescriptor {
        r#type: 0,
        physical_start: 0,
        virtual_start: 0,
        number_of_pages: 0,
        attribute: 0,
    }; 410];
    let mut memmap = MemoryMap {
        buffer_size: memmap_buf.len(),
        buffer: memmap_buf.as_mut_ptr(),
        map_size: 0,
        map_key: 0,
        descriptor_size: 0,
        descriptor_version: 0,
    };
    unsafe {
        get_memory_map((*(*system_table).boot_services).get_memory_map, &mut memmap);
    }

    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
