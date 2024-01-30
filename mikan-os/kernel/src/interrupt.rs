use core::convert::From;

pub(crate) enum DescriptorType {
    Upper8Bytes = 0,
    LDT = 2,
    TSSAvailable = 9,
    TSSBusy = 11,
    CallGate = 12,
    InterruptGate = 14,
    TrapGate = 15,
}

impl From<u8> for DescriptorType {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Upper8Bytes,
            2 => Self::LDT,
            9 => Self::TSSAvailable,
            11 => Self::TSSBusy,
            12 => Self::CallGate,
            14 => Self::InterruptGate,
            15 => Self::TrapGate,
            _ => panic!(),
        }
    }
}

#[repr(packed)]
#[derive(Clone, Copy)]
pub(crate) union InterruptDescriptorAttribute {
    data: u16,
    bits: InterruptDescriptorAttributeBits,
}

impl InterruptDescriptorAttribute {
    pub(crate) const fn const_default() -> Self {
        Self { data: 0 }
    }

    pub(crate) const fn new(
        r#type: DescriptorType,
        descriptor_privilege_level: u8,
        present: bool,
    ) -> Self {
        Self {
            bits: InterruptDescriptorAttributeBits::new(
                r#type,
                descriptor_privilege_level,
                present,
            ),
        }
    }

    fn bits(&self) -> InterruptDescriptorAttributeBits {
        unsafe { self.bits }
    }
}

impl Default for InterruptDescriptorAttribute {
    fn default() -> Self {
        Self { data: 0 }
    }
}

#[repr(packed)]
#[derive(Debug, Clone, Copy)]
struct InterruptDescriptorAttributeBits {
    etc_1: u8,
    etc_2: u8,
}

impl InterruptDescriptorAttributeBits {
    const fn new(r#type: DescriptorType, descriptor_privilege_level: u8, present: bool) -> Self {
        let etc_2 =
            (if present { 1 } else { 0 } << 7) | (descriptor_privilege_level << 5) | r#type as u8;
        Self { etc_1: 0, etc_2 }
    }

    fn interrupt_stack_table(&self) -> u8 {
        self.etc_1 & 0x07
    }

    fn r#type(&self) -> DescriptorType {
        DescriptorType::from(self.etc_2 & 0x0f)
    }

    fn descriptor_privilege_level(&self) -> u8 {
        (self.etc_2 >> 5) & 0x03
    }

    fn present(&self) -> bool {
        (self.etc_2 >> 7) & 0x01 == 1
    }
}

#[repr(packed)]
#[derive(Default, Clone, Copy)]
pub(crate) struct InterruptDescriptor {
    offset_low: u16,
    segment_selector: u16,
    attr: InterruptDescriptorAttribute,
    offset_middle: u16,
    offset_high: u32,
    _reserved: u32,
}

impl InterruptDescriptor {
    pub(crate) const fn const_default() -> Self {
        Self {
            offset_low: 0,
            segment_selector: 0,
            attr: InterruptDescriptorAttribute::const_default(),
            offset_middle: 0,
            offset_high: 0,
            _reserved: 0,
        }
    }

    pub(crate) fn set_idt_entry(
        &mut self,
        attr: InterruptDescriptorAttribute,
        offset: u64,
        segment_selector: u16,
    ) {
        self.attr = attr;
        self.offset_low = offset as u16;
        self.offset_middle = (offset >> 16) as u16;
        self.offset_high = (offset >> 32) as u32;
        self.segment_selector = segment_selector;
    }
}

pub(crate) fn notify_end_of_interrupt() {
    let end_of_interrupt = 0xfee0_00b0 as *mut u32;
    unsafe {
        end_of_interrupt.write_volatile(0);
    }
}

pub(crate) enum InterruptVector {
    XHCI = 0x40,
}

pub(crate) struct InterruptFrame {
    rip: u64,
    cs: u64,
    rflags: u64,
    rsp: u64,
    ss: u64,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(u32)]
pub(crate) enum MessageType {
    InteruptXHCI,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) struct Message {
    r#type: MessageType,
}

impl Message {
    pub(crate) fn new(r#type: MessageType) -> Self {
        Self { r#type }
    }

    pub(crate) fn r#type(&self) -> MessageType {
        self.r#type
    }
}
