#![no_std]
#![no_main]

mod chars;
mod graphics;

use crate::chars::*;
use core::{
    arch::asm,
    fmt::Write,
    mem::{size_of, transmute},
    slice,
};
use graphics::FrameBufferConfig;
use graphics::GraphicsInfo;
use log::error;
use uefi::{
    data_types::Identify,
    prelude::*,
    proto::{
        console::gop::{GraphicsOutput, PixelFormat},
        loaded_image::LoadedImage,
        media::{
            file::{Directory, File, FileAttribute, FileInfo, FileMode, RegularFile},
            fs::SimpleFileSystem,
        },
    },
    table::{
        boot::{
            AllocateType, MemoryDescriptor, MemoryMap, MemoryType, OpenProtocolAttributes,
            OpenProtocolParams, SearchType,
        },
        runtime::Time,
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

/// 画面出力情報を取得する。
fn get_gop_info(
    image_handle: Handle,
    system_table: &mut SystemTable<Boot>,
) -> uefi::Result<GraphicsInfo> {
    // GOP を操作するためのオブジェクト
    let gop_handles = system_table
        .boot_services()
        .locate_handle_buffer(SearchType::ByProtocol(&GraphicsOutput::GUID))?;

    // GOP を取得
    let mut gop = unsafe {
        system_table
            .boot_services()
            .open_protocol::<GraphicsOutput>(
                OpenProtocolParams {
                    handle: (*gop_handles)[0],
                    agent: image_handle,
                    controller: None,
                },
                OpenProtocolAttributes::GetProtocol,
            )?
    };

    let pixel_info = match gop.get() {
        None => return Err(uefi::Error::new(Status::ABORTED, ())),
        Some(gop) => gop.current_mode_info(),
    };
    error!(
        "Resolution: {}x{}",
        pixel_info.resolution().0,
        pixel_info.resolution().1
    );

    Ok(GraphicsInfo {
        pixel_info,
        frame_buffer_base: gop.frame_buffer().as_mut_ptr() as usize,
        frame_buffer_size: gop.frame_buffer().size(),
    })
}

/// ピクセルのデータ形式情報を文字列にする。
fn get_pixel_format_unicode(fmt: PixelFormat) -> &'static CStr16 {
    match fmt {
        PixelFormat::Rgb => cstr16!("PixelRedGreenBlueReserved8bitPerColor"),
        PixelFormat::Bgr => cstr16!("PixelBlueGreenRedReserved8BitPerColor"),
        PixelFormat::Bitmask => cstr16!("PixelBitMask"),
        PixelFormat::BltOnly => cstr16!("PixelBltOnly"),
    }
}

#[entry]
fn efi_main(image_handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    // 恐らく log を使えるようにしているのではないか
    match uefi_services::init(&mut system_table) {
        Err(_) => {
            system_table
                .stderr()
                .output_string(cstr16!("Failed to initialize\r\n"))
                // 流石にこれの失敗はもう無視するしかない
                .unwrap_or_default();
            halt()
        }
        Ok(_) => (),
    }

    match system_table
        .stdout()
        .output_string(cstr16!("Hello, World!\r\n"))
    {
        Err(e) => {
            error!("Failed to print: {}", e);
            halt();
        }
        Ok(_) => (),
    }

    // メモリマップの取得
    let mut memmap_buf = [0u8; 4096 * 4];
    let memmap = match get_memory_map(system_table.boot_services(), &mut memmap_buf) {
        Err(e) => {
            error!("Failed to get memmap: {}", e);
            halt();
        }
        Ok(map) => map,
    };

    // ルートディレクトリ操作用のオブジェクトの取得
    let mut root_dir = match open_root_dir(system_table.boot_services(), image_handle) {
        Err(e) => {
            error!("Failed to open root dir: {}", e);
            halt();
        }
        Ok(dir) => dir,
    };

    // メモリマップ保存用ファイルを操作するオブジェクトの取得
    let mut memmap_file = match root_dir.open(
        cstr16!("\\memmap"),
        FileMode::CreateReadWrite,
        FileAttribute::empty(),
    ) {
        Err(e) => {
            error!("Failed to open a file: {}", e);
            halt();
        }
        Ok(file) => match file.into_regular_file() {
            None => {
                // これは流石に起こらないと思うが、一応
                error!("Opend file isn't a regular file");
                halt();
            }
            Some(file) => file,
        },
    };

    // メモリマップを上で取得したファイルに保存する
    let _ = save_memory_map(&mut system_table, &memmap, &mut memmap_file);
    memmap_file.close();

    // 画面情報の取得
    let graphics_info = match get_gop_info(image_handle, &mut system_table) {
        Err(e) => {
            error!("Failed to get gop info: {}", e);
            halt();
        }
        Ok(info) => info,
    };

    // 画面情報の表示
    let mut buf16 = [0u16; 128];
    let mut str16_buf = Str16Buf::new(&mut buf16);
    match write!(
        str16_buf,
        "Resolution: {}x{}, Pixel Format: {}, {} pixels/line\r\n",
        graphics_info.pixel_info.resolution().0,
        graphics_info.pixel_info.resolution().1,
        get_pixel_format_unicode(graphics_info.pixel_info.pixel_format()),
        graphics_info.pixel_info.stride()
    ) {
        Err(e) => {
            error!("Failed to write on the buffer: {}", e);
            halt()
        }
        Ok(_) => (),
    };
    match system_table.stdout().output_string(str16_buf.into_cstr16()) {
        Err(e) => {
            error!("Failed to print: {}", e);
            halt();
        }
        Ok(_) => (),
    };

    // `\kernel.elf` を開く
    let mut kernel_file =
        match root_dir.open(cstr16!("\\kernel"), FileMode::Read, FileAttribute::empty()) {
            Err(e) => {
                error!("Failed to open kernel: {}", e);
                halt();
            }
            Ok(file) => file,
        };

    // カーネルファイル情報を取得
    const FILE_INFO_SIZE: usize = size_of::<u64>() * 2
        + size_of::<Time>() * 3
        + size_of::<FileAttribute>()
        // （恐らく）ここまでがファイル名以外の情報のためのサイズ
        // ここからはファイル名のための情報
        // 文字スライスが null 終端されていない代わりに、長さの情報を持っている（と思われる）
        + size_of::<usize>()
        + size_of::<u16>() * "\\kernel".len();
    // ファイル情報保持のためのバッファ
    let mut file_info_buffer = [0u8; FILE_INFO_SIZE];
    let file_info = match kernel_file.get_info::<FileInfo>(&mut file_info_buffer) {
        Err(e) => {
            error!("Failed to get kernel info: {}", e);
            halt();
        }
        Ok(info) => info,
    };

    let kernel_file_size = file_info.file_size();

    // ページの割り当て
    let kernel_base_addr = 0x100000;
    match system_table.boot_services().allocate_pages(
        AllocateType::Address(kernel_base_addr),
        MemoryType::LOADER_DATA,
        ((kernel_file_size + 0xfff) / 0x1000) as usize, // ページサイズは 4 KiB
    ) {
        Err(e) => {
            error!("Failed to allocate pages: {}", e);
            halt();
        }
        Ok(_) => (),
    };

    // メモリにカーネルをロード
    let mut buf = unsafe {
        slice::from_raw_parts_mut(kernel_base_addr as *mut u8, kernel_file_size as usize)
    };
    match kernel_file.into_regular_file() {
        None => {
            error!("kernel isn't a regular file");
            halt();
        }
        Some(mut file) => match file.read(&mut buf) {
            Err(e) => {
                error!("Failed to load the kernel: {}", e);
                halt();
            }
            Ok(_) => (),
        },
    }

    // 読み込んだ位置、バイト数を表示
    str16_buf.clear();
    match write!(
        str16_buf,
        "Kernel: 0x{:0x} ({} bytes)\r\n",
        kernel_base_addr, kernel_file_size
    ) {
        Err(e) => {
            error!("Failed to write on the buffer: {}", e);
            halt();
        }
        Ok(_) => (),
    };
    match system_table.stdout().output_string(str16_buf.into_cstr16()) {
        Err(e) => {
            error!("Failed to print: {}", e);
            halt();
        }
        Ok(_) => (),
    };
    error!("fb_base: 0x{:x}", graphics_info.frame_buffer_base);

    // UEFI のブートサービスを終了する
    let _ = system_table.exit_boot_services(MemoryType(0));

    // カーネルの呼び出し
    // ELF ファイルの 24 byte 目から 64 bit でエントリーポイントの番地が書いてある
    let _entry_addr = unsafe { *((kernel_base_addr + 24) as *const u64) };

    let frame_buffer = graphics_info.frame_buffer_base;
    let pixels_per_scan_line = graphics_info.pixel_info.stride();
    let (horizontal_resolution, vertical_resolution) = graphics_info.pixel_info.resolution();
    let pixel_format = match graphics_info.pixel_info.pixel_format() {
        PixelFormat::Rgb => graphics::PixelFormat::Rgb,
        PixelFormat::Bgr => graphics::PixelFormat::Bgr,
        _ => {
            error!(
                "Unimplemented pixel format: {:?}",
                graphics_info.pixel_info.pixel_format()
            );
            halt();
        }
    };
    let config = FrameBufferConfig {
        frame_buffer,
        pixels_per_scan_line,
        horizontal_resolution,
        vertical_resolution,
        pixel_format,
    };

    let entry_point: extern "sysv64" fn(FrameBufferConfig) =
        unsafe { transmute(kernel_base_addr + 0x0120) };
    entry_point(config);

    loop {}
}

fn halt() -> ! {
    unsafe {
        loop {
            asm!("hlt");
        }
    }
}
