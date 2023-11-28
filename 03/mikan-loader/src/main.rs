#![no_std]
#![no_main]

mod chars;

use crate::chars::*;
use core::{fmt::Write, mem::size_of};
use uefi::{
    prelude::*,
    proto::{
        loaded_image::LoadedImage,
        media::{
            file::{Directory, File, FileAttribute, FileMode, RegularFile},
            fs::SimpleFileSystem,
        },
    },
    table::boot::{
        MemoryDescriptor, MemoryMap, MemoryType, OpenProtocolAttributes, OpenProtocolParams,
    },
    CStr16,
};

/// メモリマップを渡されたファイルに保存する。
fn save_memory_map(
    system_table: &mut SystemTable<Boot>,
    map: &MemoryMap,
    file: &mut RegularFile,
) -> Status {
    // ファイルに書き込むヘッダ
    let header = b"Index, Type, Type(name), PhysicalStart, NumberOfPages, Attribute\n";
    let _ = file.write(header);

    // フォーマットしてコンソールに出力
    // フォーマットするには、それ用のバッファを作っておかなければならない
    let mut buf16 = [0u16; 128];
    let mut str16_buf = Str16Buf::new(&mut buf16);

    let _ = write!(
        str16_buf,
        "map->buffer = {:08x}, map->map_size = {:08x}\r\n",
        map as *const MemoryMap as usize,
        size_of::<MemoryDescriptor>() * map.entries().count()
    );
    let _ = system_table.stdout().output_string(str16_buf.into_cstr16());

    let mut i = 0;
    let mut entries = map.entries();
    // メモリマップの各エントリを書き出す
    while let Some(desc) = entries.next() {
        // フォーマットするためのバッファ
        let mut buf8 = [0u8; 256];
        let mut str8_buf = Str8Buf::new(&mut buf8);

        let _ = write!(
            str8_buf,
            "{}, {}, {}, {:08x}, {}, {}\n",
            i,
            desc.ty.0,
            get_memory_type_unicode(desc.ty),
            desc.phys_start,
            desc.page_count,
            desc.att.bits()
        );

        let _ = file.write(str8_buf.get());
        i += 1;
    }

    Status::SUCCESS
}

/// メモリマップを取得してそれを [MemoryMap] として返す。
fn get_memory_map<'a>(services: &BootServices, map: &'a mut [u8]) -> uefi::Result<MemoryMap<'a>> {
    if map.len() == 0 {
        return Err(uefi::Error::new(Status::BUFFER_TOO_SMALL, ()));
    }

    Ok(services.memory_map(map)?)
}

/// メモリのタイプ情報から、意味を表す 16 bit 文字列を返す。
fn get_memory_type_unicode(r#type: MemoryType) -> &'static CStr16 {
    match r#type {
        MemoryType::RESERVED => cstr16!("EfiReservedMemoryType"),
        MemoryType::LOADER_CODE => cstr16!("EfiLoaderCode"),
        MemoryType::LOADER_DATA => cstr16!("EfiLoaderData"),
        MemoryType::BOOT_SERVICES_CODE => cstr16!("EfiBootServicesCode"),
        MemoryType::BOOT_SERVICES_DATA => cstr16!("EfiBootServicesData"),
        MemoryType::RUNTIME_SERVICES_CODE => cstr16!("EfiRuntimeServicesCode"),
        MemoryType::RUNTIME_SERVICES_DATA => cstr16!("EfiRuntimeServicesData"),
        MemoryType::CONVENTIONAL => cstr16!("EfiConventionalMemory"),
        MemoryType::UNUSABLE => cstr16!("EfiUnusableMemory"),
        MemoryType::ACPI_RECLAIM => cstr16!("EfiACPIReclaimMemory"),
        MemoryType::ACPI_NON_VOLATILE => cstr16!("EfiAcpiMemoryNVS"),
        MemoryType::MMIO => cstr16!("EfiMemoryMappedIO"),
        MemoryType::MMIO_PORT_SPACE => cstr16!("EfiMemoryMappedIOPortSpace"),
        MemoryType::PAL_CODE => cstr16!("EfiPalCode"),
        MemoryType::PERSISTENT_MEMORY => cstr16!("EfiPersistentMemory"),
        _ => cstr16!("InvalidMemoryType"),
    }
}

/// ルートディレクトリを操作するオブジェクト（[Directory]）を返す。
/// 失敗した場合は [uefi::Error] が返る。
fn open_root_dir(services: &BootServices, image_handle: Handle) -> uefi::Result<Directory> {
    // 恐らく、ロードしているデバイスを操作するオブジェクトの取得
    let loaded_image = match match unsafe {
        services.open_protocol::<LoadedImage>(
            OpenProtocolParams {
                handle: image_handle,
                agent: image_handle,
                controller: None,
            },
            OpenProtocolAttributes::GetProtocol,
        )?
    }
    .get_mut()
    {
        None => return Err(uefi::Error::new(Status::ABORTED, ())),
        Some(proto) => proto,
    }
    .device()
    {
        None => return Err(uefi::Error::new(Status::ABORTED, ())),
        Some(handle) => handle,
    };

    // デバイスのファイルシステム操作用オブジェクトの取得
    let binding = unsafe {
        services.open_protocol::<SimpleFileSystem>(
            OpenProtocolParams {
                handle: loaded_image,
                agent: image_handle,
                controller: None,
            },
            OpenProtocolAttributes::GetProtocol,
        )?
    };
    let fs = match binding.get_mut() {
        None => return Err(uefi::Error::new(Status::ABORTED, ())),
        Some(proto) => proto,
    };

    // ルートディレクトリを開いて返す
    Ok(fs.open_volume()?)
}

#[entry]
fn efi_main(image_handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    // 恐らく log を使えるようにしているのではないか
    uefi_services::init(&mut system_table).unwrap();

    system_table
        .stdout()
        .output_string(cstr16!("Hello, World!\r\n"))
        .unwrap();

    // メモリマップの取得
    let mut memmap_buf = [0u8; 4096 * 4];
    let memmap = match get_memory_map(system_table.boot_services(), &mut memmap_buf) {
        Err(e) => return e.status(),
        Ok(map) => map,
    };

    // ルートディレクトリ操作用のオブジェクトの取得
    let mut root_dir = match open_root_dir(system_table.boot_services(), image_handle) {
        Err(e) => return e.status(),
        Ok(dir) => dir,
    };

    // メモリマップ保存用ファイルを操作するオブジェクトの取得
    let mut memmap_file = match root_dir.open(
        cstr16!("\\memmap"),
        FileMode::CreateReadWrite,
        FileAttribute::empty(),
    ) {
        Err(e) => return e.status(),
        Ok(file) => file.into_regular_file().unwrap(),
    };

    // メモリマップを上で取得したファイルに保存する
    let _ = save_memory_map(&mut system_table, &memmap, &mut memmap_file);
    memmap_file.close();

    let _ = system_table.stdout().output_string(cstr16!("All done\r\n"));

    loop {}
}
