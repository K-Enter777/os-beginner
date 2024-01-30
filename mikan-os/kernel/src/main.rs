#![no_std]
#![no_main]

mod asmfunc;
mod console;
mod error;
mod font;
mod font_data;
mod frame_buffer_config;
mod graphics;
mod interrupt;
mod logger;
mod mouse;
mod pci;
mod placement;
mod queue;
mod string;
mod usb;

use console::Console;
use core::{arch::asm, cell::OnceCell, mem::size_of, panic::PanicInfo};
use frame_buffer_config::{FrameBufferConfig, PixelFormat};
use graphics::{
    BgrResv8BitPerColorPixelWriter, PixelColor, PixelWriter, RgbResv8BitPerColorPixelWriter,
    Vector2D,
};
use interrupt::{notify_end_of_interrupt, InterruptFrame, Message};
use mouse::MouseCursor;
use pci::Device;
use placement::new_mut_with_buf;
use queue::ArrayQueue;
use uefi::table::boot::{MemoryMap, MemoryType};

use crate::{
    asmfunc::{get_cs, load_idt},
    interrupt::{InterruptDescriptor, InterruptDescriptorAttribute, InterruptVector, MessageType},
    logger::{set_log_level, LogLevel},
    usb::{Controller, HIDMouseDriver},
};

/// デスクトップ背景の色
const DESKTOP_BG_COLOR: PixelColor = PixelColor::new(45, 118, 237);
/// デスクトップ前景の色
const DESKTOP_FG_COLOR: PixelColor = PixelColor::new(255, 255, 255);

const PIXEL_WRITER_SIZE: usize = size_of::<RgbResv8BitPerColorPixelWriter>();
static mut PIXEL_WRITER_BUF: [u8; PIXEL_WRITER_SIZE] = [0u8; PIXEL_WRITER_SIZE];
static mut CONSOLE: OnceCell<Console> = OnceCell::new();

static mut IDT: [InterruptDescriptor; 256] = [InterruptDescriptor::const_default(); 256];

#[macro_export]
macro_rules! printk {
    ($($arg:tt)*) => {
        unsafe {
            use core::fmt::Write;
            match $crate::CONSOLE.get_mut() {
                Some(console) => write!(console, $($arg)*).unwrap(),
                None => $crate::halt(),
            }
        }
    };
}

#[macro_export]
macro_rules! printkln {
    () => ($crate::printk!("\n"));
    ($($arg:tt)*) => ($crate::printk!("{}\n", format_args!($($arg)*)));
}

static mut MOUSE_CURSOR: OnceCell<MouseCursor> = OnceCell::new();

fn mouse_observer(displacement_x: i8, displacement_y: i8) {
    let cursor = match unsafe { MOUSE_CURSOR.get_mut() } {
        None => halt(),
        Some(cursor) => cursor,
    };
    cursor.move_relative(Vector2D::new(displacement_x as u32, displacement_y as u32));
}

fn switch_ehci2xhci(xhc_dev: &Device) {
    let mut intel_ehc_exist = false;
    let num_device = *pci::NUM_DEVICES.lock().borrow();
    let devices = pci::DEVICES.lock();
    let devices = devices.borrow();
    for i in 0..num_device {
        if devices[i].unwrap().class_code().r#match(0x0c, 0x03, 0x20)
            && devices[i].unwrap().read_vendor_id() == 0x8086
        {
            intel_ehc_exist = true;
            break;
        }
    }
    if !intel_ehc_exist {
        return;
    }

    let superspeed_ports = xhc_dev.read_conf_reg(0xdc);
    xhc_dev.write_conf_reg(0xd8, superspeed_ports);
    let ehci2xhci_ports = xhc_dev.read_conf_reg(0xd4);
    xhc_dev.write_conf_reg(0xd0, ehci2xhci_ports);
    log!(
        LogLevel::Debug,
        "switch_ehci2xhci: SS = {:02x}, xHCI = {:02x}",
        superspeed_ports,
        ehci2xhci_ports
    );
}

static mut XHC: OnceCell<Controller> = OnceCell::new();

const MAIN_QUEUE_BUF_SIZE: usize = size_of::<Message>() * 32;
static mut MAIN_QUEUE_BUF: [u8; MAIN_QUEUE_BUF_SIZE] = [0; MAIN_QUEUE_BUF_SIZE];
static mut MAIN_QUEUE: OnceCell<ArrayQueue<Message>> = OnceCell::new();

#[custom_attribute::interrupt]
fn int_handler_xhci(_frame: &InterruptFrame) {
    let main_queue = unsafe { MAIN_QUEUE.get_mut() }.unwrap();
    main_queue.push(Message::new(MessageType::InteruptXHCI));
    notify_end_of_interrupt();
}

#[no_mangle]
pub extern "sysv64" fn kernel_entry(frame_buffer_config: FrameBufferConfig, memory_map: MemoryMap) {
    let pixel_writer: &mut dyn PixelWriter = match frame_buffer_config.pixel_format {
        PixelFormat::Rgb => {
            match unsafe {
                new_mut_with_buf(
                    RgbResv8BitPerColorPixelWriter::new(frame_buffer_config),
                    &mut PIXEL_WRITER_BUF,
                )
            } {
                Err(_size) => halt(),
                Ok(writer) => writer,
            }
        }
        PixelFormat::Bgr => {
            match unsafe {
                new_mut_with_buf(
                    BgrResv8BitPerColorPixelWriter::new(frame_buffer_config),
                    &mut PIXEL_WRITER_BUF,
                )
            } {
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
    unsafe {
        CONSOLE.get_or_init(|| Console::new(pixel_writer, &DESKTOP_FG_COLOR, &DESKTOP_BG_COLOR));
    }

    // welcome 文
    printk!("Welcome to MikanOS!\n");
    set_log_level(LogLevel::Warn);

    let available_memory_types = [
        MemoryType::BOOT_SERVICES_CODE,
        MemoryType::BOOT_SERVICES_DATA,
        MemoryType::CONVENTIONAL,
    ];

    printkln!("memory_map: {:p}", &memory_map);
    for desc in memory_map.entries() {
        for mem_ty in available_memory_types {
            if desc.ty == mem_ty {
                printkln!(
                    "type = {}, phys = {:08x} - {:08x}, pages = {}, attr = {:08x}",
                    desc.ty.0,
                    desc.phys_start,
                    desc.phys_start + desc.page_count * 4096 - 1,
                    desc.page_count,
                    desc.att
                );
            }
        }
    }

    // マウスカーソルの生成
    unsafe {
        MOUSE_CURSOR.get_or_init(|| {
            MouseCursor::new(pixel_writer, DESKTOP_BG_COLOR, Vector2D::new(300, 200))
        });
    }

    // 割り込みキューの初期化
    unsafe { MAIN_QUEUE.get_or_init(|| ArrayQueue::new(&mut MAIN_QUEUE_BUF)) };

    // デバイス一覧の表示
    let err = pci::scan_all_bus();
    log!(LogLevel::Debug, "scan_all_bus: {}", err);

    let mut xhc_dev = None;
    {
        let devices = pci::DEVICES.lock();
        let devices = devices.borrow();
        let num_devices = *pci::NUM_DEVICES.lock().borrow();
        for i in 0..num_devices {
            let dev = devices[i].unwrap();
            let vendor_id = dev.read_vendor_id();
            let class_code = pci::read_class_code(dev.bus(), dev.device(), dev.function());
            log!(
                LogLevel::Debug,
                "{}.{}.{}: vend {:04x}, class {:08x}, head {:02x}",
                dev.bus(),
                dev.device(),
                dev.function(),
                vendor_id,
                class_code,
                dev.header_type()
            );
        }

        // Intel 製を優先して xHC を探す
        for i in 0..num_devices {
            if devices[i].unwrap().class_code().r#match(0x0c, 0x03, 0x30) {
                xhc_dev = devices[i];

                if 0x8086 == xhc_dev.unwrap().read_vendor_id() {
                    break;
                }
            }
        }

        if xhc_dev.is_some() {
            let xhc_dev = xhc_dev.unwrap();
            log!(
                LogLevel::Info,
                "xHC has been found: {}.{}.{}",
                xhc_dev.bus(),
                xhc_dev.device(),
                xhc_dev.function()
            );
        }
    }
    let mut xhc_dev = xhc_dev.unwrap();

    let cs = unsafe { get_cs() };
    unsafe {
        IDT[InterruptVector::XHCI as usize].set_idt_entry(
            InterruptDescriptorAttribute::new(interrupt::DescriptorType::InterruptGate, 0, true),
            int_handler_xhci as *const fn() as u64,
            cs,
        );
        load_idt(
            (size_of::<InterruptDescriptor>() * IDT.len()) as u16 - 1,
            IDT.as_ptr() as u64,
        )
    }

    let bsp_local_apic_id = (unsafe { *(0xfee0_0020 as *const u32) } >> 24) as u8;
    xhc_dev.configure_msi_fixed_destination(
        bsp_local_apic_id,
        pci::MSITriggerMode::Level,
        pci::MSIDeliverMode::Fixed,
        InterruptVector::XHCI as u8,
        0,
    );
    let xhc_dev = xhc_dev;

    // xHC の BAR から情報を得る
    let xhc_bar = xhc_dev.read_bar(0);
    log!(LogLevel::Debug, "ReadBar: {}", xhc_bar.error());
    let xhc_mmio_base = *xhc_bar.value() & !0xf;
    log!(LogLevel::Debug, "xHC mmio_base = {:08x}", xhc_mmio_base);

    let mut xhc = Controller::new(xhc_mmio_base);

    if xhc_dev.read_vendor_id() == 0x8086 {
        switch_ehci2xhci(&xhc_dev);
    }
    {
        let err = xhc.initialize();
        log!(LogLevel::Debug, "xhc.initialize: {}", err);
    }

    log!(LogLevel::Info, "xHC starting");
    xhc.run();

    unsafe {
        XHC.get_or_init(|| xhc);
    }

    HIDMouseDriver::set_default_observer(mouse_observer);

    {
        let xhc = unsafe { XHC.get_mut() }.unwrap();

        for i in 1..=xhc.max_ports() {
            let mut port = xhc.port_at(i);
            log!(
                LogLevel::Debug,
                "Port {}: IsConnected={}",
                i,
                port.is_connected()
            );

            if port.is_connected() {
                let err = xhc.configure_port(&mut port);
                if (&err).into() {
                    log!(LogLevel::Error, "failed to configure port: {}", err);
                    continue;
                }
            }
        }
    }

    loop {
        unsafe { asm!("cli") };
        let main_queue = unsafe { MAIN_QUEUE.get_mut() }.unwrap();

        if main_queue.len() == 0 {
            unsafe {
                asm!("sti");
                asm!("hlt");
            }
            continue;
        }

        let msg = *main_queue.front().unwrap();
        main_queue.pop();
        unsafe { asm!("sti") };

        match msg.r#type() {
            MessageType::InteruptXHCI => {
                let xhc = unsafe { XHC.get_mut() }.unwrap();
                while xhc.primary_event_ring().has_front() {
                    let err = xhc.process_event();
                    if (&err).into() {
                        log!(LogLevel::Error, "Error while process_evnet: {}", err);
                    }
                }
            }
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    printkln!("{}", info);
    halt()
}

fn halt() -> ! {
    loop {
        unsafe {
            asm!("hlt");
        }
    }
}
