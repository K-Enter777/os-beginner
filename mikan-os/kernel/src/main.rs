#![no_std]
#![no_main]

extern crate alloc;

mod asmfunc;
mod bitfield;
mod console;
mod error;
mod font;
mod font_data;
mod frame_buffer_config;
mod graphics;
mod interrupt;
mod logger;
mod memory_manager;
mod memory_map;
mod mouse;
mod paging;
mod pci;
mod segment;
mod sync;
mod usb;
mod x86_descriptor;

use alloc::{boxed::Box, collections::VecDeque};
use console::Console;
use core::{arch::asm, mem::size_of, panic::PanicInfo};
use frame_buffer_config::{FrameBufferConfig, PixelFormat};
use graphics::{
    BgrResv8BitPerColorPixelWriter, PixelColor, PixelWriter, RgbResv8BitPerColorPixelWriter,
    Vector2D,
};
use interrupt::{notify_end_of_interrupt, InterruptFrame, Message};
use mouse::MouseCursor;
use pci::Device;
use sync::{OnceRwLock, RwLock};
use uefi::table::boot::MemoryMap;

use crate::{
    asmfunc::{get_cs, load_idt, set_cs_ss, set_ds_all},
    interrupt::{InterruptDescriptor, InterruptDescriptorAttribute, InterruptVector, MessageType},
    logger::{set_log_level, LogLevel},
    usb::{Controller, HIDMouseDriver},
};

/// カーネル用スタック
#[repr(align(16))]
struct KernelStack {
    _buf: [u8; 1024 * 1024],
}
impl KernelStack {
    const fn new() -> Self {
        Self {
            _buf: [0; 1024 * 1024],
        }
    }
}
#[allow(unused)]
#[no_mangle]
static KERNEL_MAIN_STACK: KernelStack = KernelStack::new();

/// ピクセル描画を担う。
static PIXEL_WRITER: OnceRwLock<Box<dyn PixelWriter + Send>> = OnceRwLock::new();

/// デスクトップ背景の色
const DESKTOP_BG_COLOR: PixelColor = PixelColor::new(45, 118, 237);
/// デスクトップ前景の色
const DESKTOP_FG_COLOR: PixelColor = PixelColor::new(255, 255, 255);

/// コンソール処理を担う。
static CONSOLE: OnceRwLock<Console> = OnceRwLock::new();

static IDT: RwLock<[InterruptDescriptor; 256]> =
    RwLock::new([InterruptDescriptor::const_default(); 256]);

#[macro_export]
macro_rules! printk {
    ($($arg:tt)*) => {
        {
            use core::fmt::Write;
            write!($crate::CONSOLE.write(), $($arg)*).unwrap();
        }
    };
}

#[macro_export]
macro_rules! printkln {
    () => ($crate::printk!("\n"));
    ($($arg:tt)*) => ($crate::printk!("{}\n", format_args!($($arg)*)));
}

static MOUSE_CURSOR: OnceRwLock<MouseCursor> = OnceRwLock::new();

fn mouse_observer(displacement_x: i8, displacement_y: i8) {
    MOUSE_CURSOR
        .write()
        .move_relative(Vector2D::new(displacement_x as u32, displacement_y as u32));
}

fn switch_ehci2xhci(xhc_dev: &Device) {
    let mut intel_ehc_exist = false;
    let devices = pci::DEVICES.read();
    for device in &*devices {
        if device.class_code().r#match(0x0c, 0x03, 0x20) && device.read_vendor_id() == 0x8086 {
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

static XHC: OnceRwLock<Controller> = OnceRwLock::new();

static MAIN_QUEUE: RwLock<VecDeque<Message>> = RwLock::new(VecDeque::new());

#[custom_attribute::interrupt]
fn int_handler_xhci(_frame: &InterruptFrame) {
    MAIN_QUEUE
        .write()
        .push_back(Message::new(MessageType::InteruptXHCI));
    notify_end_of_interrupt();
}

// この呼び出しの前にスタック領域を変更するため、でかい構造体をそのまま渡せなくなる
// それを避けるために参照で渡す
#[custom_attribute::kernel_entry(KERNEL_MAIN_STACK, 1024 * 1024)]
fn kernel_entry(
    frame_buffer_config: &'static FrameBufferConfig,
    memory_map: &'static MemoryMap,
    kernel_base: usize,
    kernel_size: usize,
) {
    // 参照元は今後使用される可能性のあるメモリ領域にあるため、コピーしておく
    let frame_buffer_config = frame_buffer_config.clone();

    // メモリアロケータの初期化
    memory_manager::GLOBAL.initialize(memory_map, kernel_base, kernel_size);

    let pixel_writer: Box<dyn PixelWriter + Send> = match frame_buffer_config.pixel_format {
        PixelFormat::Rgb => Box::new(RgbResv8BitPerColorPixelWriter::new(frame_buffer_config)),

        PixelFormat::Bgr => Box::new(BgrResv8BitPerColorPixelWriter::new(frame_buffer_config)),
    };

    let frame_width = pixel_writer.config().horizontal_resolution as u32;
    let frame_height = pixel_writer.config().vertical_resolution as u32;

    PIXEL_WRITER.init(pixel_writer);

    {
        let mut pixel_writer = PIXEL_WRITER.write();
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
    }
    // コンソールの生成
    CONSOLE.init(Console::new(
        &PIXEL_WRITER,
        &DESKTOP_FG_COLOR,
        &DESKTOP_BG_COLOR,
    ));

    // welcome 文
    printk!("Welcome to MikanOS!\n");
    set_log_level(LogLevel::Warn);

    // セグメントの設定
    segment::setup_segments();

    const KERNEL_CS: u16 = 1 << 3;
    const KERNEL_SS: u16 = 2 << 3;
    unsafe {
        set_ds_all(0);
        set_cs_ss(KERNEL_CS, KERNEL_SS);
    }

    // ページングの設定
    paging::setup_indentity_page_table();

    // マウスカーソルの生成
    MOUSE_CURSOR.init(MouseCursor::new(
        &PIXEL_WRITER,
        DESKTOP_BG_COLOR,
        Vector2D::new(300, 200),
    ));

    // デバイス一覧の表示
    let result = pci::scan_all_bus();
    log!(LogLevel::Debug, "scan_all_bus: {:?}", result);

    let mut xhc_dev = None;
    {
        let devices = pci::DEVICES.read();
        for device in &*devices {
            let dev = device;
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
        for device in &*devices {
            if device.class_code().r#match(0x0c, 0x03, 0x30) {
                xhc_dev = Some(*device);

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
    {
        let mut idt = IDT.write();
        idt[InterruptVector::XHCI as usize].set_idt_entry(
            InterruptDescriptorAttribute::new(
                x86_descriptor::SystemSegmentType::InterruptGate,
                0,
                true,
            ),
            int_handler_xhci as *const fn() as u64,
            cs,
        );
        unsafe {
            load_idt(
                (size_of::<InterruptDescriptor>() * idt.len()) as u16 - 1,
                idt.as_ptr() as u64,
            )
        }
    }

    let bsp_local_apic_id = (unsafe { *(0xfee0_0020 as *const u32) } >> 24) as u8;
    xhc_dev
        .configure_msi_fixed_destination(
            bsp_local_apic_id,
            pci::MSITriggerMode::Level,
            pci::MSIDeliverMode::Fixed,
            InterruptVector::XHCI as u8,
            0,
        )
        .unwrap();
    let xhc_dev = xhc_dev;

    // xHC の BAR から情報を得る
    let xhc_bar = xhc_dev.read_bar(0);
    log!(LogLevel::Debug, "ReadBar: {:?}", xhc_bar);
    let xhc_mmio_base = xhc_bar.unwrap() & !0xf;
    log!(LogLevel::Debug, "xHC mmio_base = {:08x}", xhc_mmio_base);

    let mut xhc = Controller::new(xhc_mmio_base);

    if xhc_dev.read_vendor_id() == 0x8086 {
        switch_ehci2xhci(&xhc_dev);
    }
    {
        let result = xhc.initialize();
        log!(LogLevel::Debug, "xhc.initialize: {:?}", result);
    }

    log!(LogLevel::Info, "xHC starting");
    xhc.run().unwrap();

    XHC.init(xhc);

    HIDMouseDriver::set_default_observer(mouse_observer);

    {
        let mut xhc = XHC.write();

        for i in 1..=xhc.max_ports() {
            let mut port = xhc.port_at(i);
            log!(
                LogLevel::Debug,
                "Port {}: IsConnected={}",
                i,
                port.is_connected()
            );

            if port.is_connected() {
                if let Err(err) = xhc.configure_port(&mut port) {
                    log!(LogLevel::Error, "failed to configure port: {}", err);
                    continue;
                }
            }
        }
    }

    loop {
        unsafe { asm!("cli") };
        let mut main_queue = MAIN_QUEUE.write();

        if main_queue.len() == 0 {
            unsafe {
                asm!("sti", "hlt");
            }
            continue;
        }

        let msg = *main_queue.front().unwrap();
        main_queue.pop_front();
        unsafe { asm!("sti") };

        match msg.r#type() {
            MessageType::InteruptXHCI => {
                let mut xhc = XHC.write();
                while xhc.primary_event_ring().has_front() {
                    if let Err(err) = xhc.process_event() {
                        log!(LogLevel::Error, "Error while process_evnet: {}", err);
                    }
                }
            }
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // FIXME: 前の改行の有無をチェックし、なければ改行を追加する
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
