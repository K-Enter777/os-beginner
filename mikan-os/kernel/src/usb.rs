use core::{
    ffi::{c_char, c_int, c_schar, c_uchar, c_ulong, c_void, CStr},
    mem::MaybeUninit,
};

use crate::error;

#[repr(C)]
pub(crate) struct Controller {
    mmio_base: c_ulong,
    cap: *const (),
    op: *const (),
    max_ports: c_uchar,
    devmgr: DeviceManager,
    cr: Ring,
    er: EventRing,
}

#[repr(C)]
pub(crate) struct DeviceManager {
    device_context_pointers: *mut *mut (), // 本当は DeviceContext**
    max_slots: c_ulong,
}

#[repr(C)]
pub(crate) struct Ring {
    buf: *mut (), // 本当は *TRB
    buf_size: c_ulong,
    cycle_bit: bool,
    write_index: c_ulong,
}

#[repr(C)]
pub(crate) struct EventRing {
    buf: *mut (), // 本当は TRB*
    buf_size: c_ulong,
    cycle_bit: bool,
    erst: *mut (),        // 本当は EventRingSegmentTableEntry*
    interrupter: *mut (), // 本当は InterrupterRegisterSet*
}

extern "C" {
    #[link_name = "_ZN3usb4xhci10ControllerC2Em"]
    fn contoller(this: *mut Controller, mmio_base: c_ulong);

    #[link_name = "_ZN3usb4xhci10Controller10InitializeEv"]
    fn contoller_initialize(this: *mut Controller) -> CxxError;

    #[link_name = "_ZN3usb4xhci10Controller3RunEv"]
    fn controller_run(this: *mut Controller) -> CxxError;

    // #[link_name = "_ZNK3usb4xhci10Controller8MaxPortsEv"]
    // fn controller_max_ports(this: *const Controller) -> c_schar;

    #[link_name = "_ZN3usb4xhci10Controller6PortAtEh"]
    fn controller_port_at(this: *mut Controller, port_num: c_uchar) -> Port;

    #[link_name = "_ZN3usb4xhci13ConfigurePortERNS0_10ControllerERNS0_4PortE"]
    fn xhci_configure_port(xhc: *mut Controller, port: *mut Port) -> CxxError;

    #[link_name = "_ZN3usb4xhci12ProcessEventERNS0_10ControllerE"]
    fn xhci_process_event(xhc: *mut Controller) -> CxxError;

    #[link_name = "_ZN3usb14HIDMouseDriver16default_observerE"]
    static mut HID_MOUSE_DRIVER_DEFAULT_OBSERVER: Function;

    #[link_name = "_ZN3usb14HIDMouseDriver18SetDefaultObserverEPFvaaE"]
    fn hid_mouse_driver_set_default_observer(observer: *const c_void);

    #[link_name = "_ZNK3usb4xhci4Port11IsConnectedEv"]
    fn port_is_connected(this: *const Port) -> bool;
}

impl Controller {
    pub(crate) fn new(mmio_base: u64) -> Self {
        let mut this = MaybeUninit::<Controller>::uninit();
        unsafe {
            contoller(this.as_mut_ptr(), mmio_base);
            this.assume_init()
        }
    }

    pub(crate) fn initialize(&mut self) -> error::Error {
        unsafe { contoller_initialize(self as *mut Self) }.into()
    }

    pub(crate) fn run(&mut self) -> error::Error {
        unsafe { controller_run(self as *mut Self) }.into()
    }

    pub(crate) fn max_ports(&self) -> u8 {
        self.max_ports
    }

    pub(crate) fn port_at(&mut self, port_num: u8) -> Port {
        unsafe { controller_port_at(self as *mut Self, port_num) }
    }

    pub(crate) fn configure_port(&mut self, port: &mut Port) -> error::Error {
        unsafe { xhci_configure_port(self as *mut Self, port as *mut Port) }.into()
    }

    pub(crate) fn process_event(&mut self) -> error::Error {
        unsafe { xhci_process_event(self as *mut Self) }.into()
    }
}

type ObserverType = fn(c_schar, c_schar);

#[repr(C)]
struct Function {
    a: [i32; 8], // 本当の構成は分からないが、32 byte 使っているのでこうしている
}

#[repr(C)]
pub(crate) struct HIDMouseDriver {
    observers: [Function; 4], // 本当は Function<ObserverType>
    num_observers: i32,
}

impl HIDMouseDriver {
    pub(crate) fn set_default_observer(observer: ObserverType) {
        unsafe {
            hid_mouse_driver_set_default_observer(observer as *const c_void);
        }
    }
}

#[repr(C)]
pub(crate) struct CxxError {
    code: error::Code,
    line: c_int,
    file: *const c_char,
}

impl Into<error::Error> for CxxError {
    fn into(self) -> error::Error {
        error::Error::new(
            self.code,
            unsafe { CStr::from_ptr(self.file) }.to_str().unwrap(),
            self.line as u32,
        )
    }
}

#[repr(C)]
pub(crate) struct Port {
    port_num: c_uchar,
    port_reg_set: *mut (), // 本当は PortRegisterSet&
}

impl Port {
    pub(crate) fn is_connected(&self) -> bool {
        unsafe { port_is_connected(self as *const Self) }
    }
}
