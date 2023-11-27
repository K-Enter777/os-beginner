#![no_std]
#![no_main]

use core::{ffi::c_void, panic::PanicInfo, ptr::null_mut};
use r_efi::{
    efi::{self, Guid, PhysicalAddress},
    protocols::{file, loaded_image, shell::FileHandle, simple_file_system},
    system::{BootOpenProtocol, OPEN_PROTOCOL_BY_HANDLE_PROTOCOL},
};
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

fn oepn_root_dir(
    open_protocol: BootOpenProtocol,
    image_handle: efi::Handle,
    root: *mut *mut file::Protocol,
) -> efi::Status {
    let loaded_image: *mut loaded_image::Protocol = null_mut();
    let fs: *mut simple_file_system::Protocol = null_mut();

    let mut guid = loaded_image::PROTOCOL_GUID;
    open_protocol(
        image_handle,
        &mut guid as *mut efi::Guid,
        loaded_image as *mut *mut c_void,
        image_handle,
        null_mut(),
        OPEN_PROTOCOL_BY_HANDLE_PROTOCOL,
    );

    let mut guid = simple_file_system::PROTOCOL_GUID;
    unsafe {
        open_protocol(
            (*loaded_image).device_handle,
            &mut guid as *mut Guid,
            fs as *mut *mut c_void,
            image_handle,
            null_mut(),
            OPEN_PROTOCOL_BY_HANDLE_PROTOCOL,
        );
    }

    unsafe { ((*fs).open_volume)(fs, root) }
}

fn save_memory_map(map: &MemoryMap, file: *mut file::Protocol) -> efi::Status {
    let buf = [0u8; 256];
    let header = b"Index, Type, Type(name), PhysicalStart, NumberOfPages, Attribute\n\0";

    let mut len = header.len();
    unsafe {
        ((*file).write)(file, &mut len as *mut usize, header.as_ptr() as *mut c_void);
    }

    // TODO: print

    let mut iter = map.buffer as PhysicalAddress;
    let mut i = 0;
    while iter < map.buffer as PhysicalAddress + map.map_size as PhysicalAddress {
        unsafe {
            let desc = *map.buffer;

            let mut buf = [0u16; 256];
        }

        iter += map.descriptor_size as PhysicalAddress;
        i += 1;
    }

    efi::Status::SUCCESS
}

#[export_name = "efi_main"]
pub extern "C" fn efi_main(
    image_handle: efi::Handle,
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

    let mut root_dir: *mut file::Protocol = null_mut();
    unsafe {
        oepn_root_dir(
            (*(*system_table).boot_services).open_protocol,
            image_handle,
            &mut root_dir as *mut *mut file::Protocol,
        );
    }

    let mut memmap_file: *mut file::Protocol = null_mut();
    let s = utf16!("\\memmap");
    unsafe {
        ((*root_dir).open)(
            root_dir,
            &mut memmap_file as *mut *mut file::Protocol,
            s.as_ptr() as *mut u16,
            file::MODE_READ | file::MODE_WRITE | file::MODE_CREATE,
            0,
        );
    }

    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
